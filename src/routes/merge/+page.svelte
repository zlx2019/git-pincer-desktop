<script lang="ts">
  // 三栏合并页(M3: 完整交互): gutter 应用/忽略、接缝连接带、撤销、批量应用、
  // 导航、中栏自由编辑(命中 chunk 记为手工解决)、Apply 落盘
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { Annotation, Compartment, Transaction, type Extension } from '@codemirror/state';
  import { EditorView, gutterLineClass, keymap, lineNumbers } from '@codemirror/view';
  import { history, historyKeymap, defaultKeymap } from '@codemirror/commands';
  import { syntaxHighlighting } from '@codemirror/language';
  import { api, type MergeChunk, type MergeSnapshot } from '$lib/api';
  import { session } from '$lib/state.svelte';
  import { toast } from '$lib/toast.svelte';
  import { largeWindow } from '$lib/win';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import {
    buildPaneDecos,
    createPane,
    ideaHighlight,
    ideaTheme,
    languageFor,
    lineRangeToPos,
    linkScroll,
    readonlyExtensions,
    type Pane,
  } from '$lib/editor';

  const path = $derived(page.url.searchParams.get('path') ?? '');

  type SideState = 'pending' | 'applied' | 'ignored';
  interface ChunkState {
    ours: SideState;
    theirs: SideState;
    edited: boolean;
  }
  interface UndoEntry {
    id: number;
    text: string;
    state: ChunkState;
  }

  const SEAM = 44;
  /// chunk 操作事务标记: 与手工编辑区分, 且不进 CM6 历史(有独立撤销栈)
  const chunkOp = Annotation.define<boolean>();

  // 接缝按钮图标(静态字面量, {@html} 安全); 线条风格与菜单页图标一致, 对齐 IDEA 的细线灰色 glyph
  const IC = {
    applyL:
      '<svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 4.5 7.5 8 4 11.5M8.5 4.5 12 8l-3.5 3.5"/></svg>',
    applyR:
      '<svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M12 4.5 8.5 8l3.5 3.5M7.5 4.5 4 8l3.5 3.5"/></svg>',
    ignore:
      '<svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="m4.5 4.5 7 7M11.5 4.5l-7 7"/></svg>',
    revert:
      '<svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M3.5 3.5v4h4"/><path d="M3.8 7.2a4.9 4.9 0 1 0 1.1-3"/></svg>',
  };

  let snap: MergeSnapshot | null = $state(null);
  let showWords = $state(true);
  let applyAsk = $state(false);
  let states = $state<ChunkState[]>([]);
  let resultRanges = $state<{ from: number; to: number }[]>([]);
  let undoStack = $state<UndoEntry[]>([]);
  let curIdx = $state(-1);
  let scrollTick = $state(0);
  let mounted = $state(false);

  let leftEl: HTMLElement | undefined = $state();
  let resultEl: HTMLElement | undefined = $state();
  let rightEl: HTMLElement | undefined = $state();
  let views: EditorView[] = [];
  let unlink: (() => void) | null = null;
  let leftLines: string[] = [];
  let rightLines: string[] = [];
  let baseLines: string[] = [];
  const leftComp = new Compartment();
  const centerComp = new Compartment();
  const rightComp = new Compartment();
  let decoScheduled = false;

  const totalResultLines = $derived.by(() => (snap ? snap.result.split('\n').length : 1));
  const changesLeft = $derived.by(() => states.filter((_, i) => !resolved(i)).length);
  const conflictsLeft = $derived.by(() =>
    snap ? snap.chunks.filter((c) => c.kind === 'conflict' && !resolved(c.id)).length : 0
  );

  onMount(() => {
    largeWindow().catch(() => {});
    if (!session.info) {
      goto('/');
      return;
    }
    load();
    return () => {
      unlink?.();
      for (const v of views) {
        v.scrollDOM.removeEventListener('scroll', bumpTick);
        v.destroy();
      }
    };
  });

  /** rAF 节流的接缝重绘触发: 原生 scroll 事件逐帧驱动, 消除按钮相对内容的漂移感 */
  let rafPending = false;
  function bumpTick() {
    if (rafPending) return;
    rafPending = true;
    requestAnimationFrame(() => {
      rafPending = false;
      scrollTick += 1;
    });
  }

  /** 接缝不是滚动容器: 滚轮转发给中栏(同步联动其余两栏) */
  function seamWheel(e: WheelEvent) {
    views[1]?.scrollDOM.scrollBy({ top: e.deltaY });
  }

  /// CM6 的行高测量是异步的(初次布局/字体就绪后才准), 几何变化时必须触发接缝重绘,
  /// 否则按初始估算行高算出的按钮/色带位置会随行号累积偏移
  const geoTick = EditorView.updateListener.of((u) => {
    if (u.geometryChanged) bumpTick();
  });

  /** 拉取快照并构建三栏 */
  async function load() {
    try {
      const s = await api.openMerge(path);
      snap = s;
      leftLines = s.left.split('\n');
      rightLines = s.right.split('\n');
      baseLines = s.result.split('\n');
      states = s.chunks.map(() => ({ ours: 'pending', theirs: 'pending', edited: false }));
      const lang = await languageFor(s.path);
      await new Promise((r) => requestAnimationFrame(r));
      if (!leftEl || !resultEl || !rightEl) return;

      const left = createPane(leftEl, s.left, [...readonlyExtensions(lang), geoTick, leftComp.of([])]);
      const right = createPane(rightEl, s.right, [
        ...readonlyExtensions(lang),
        geoTick,
        rightComp.of([]),
      ]);
      const result = createPane(resultEl, s.result, centerExtensions(lang));
      views = [left, result, right];
      resultRanges = s.chunks.map((c) => lineRangeToPos(result.state.doc, c.resultRange));
      unlink = linkScroll([
        { view: left, anchors: anchorsOf(s, 'left') },
        { view: result, anchors: anchorsOf(s, 'result') },
        { view: right, anchors: anchorsOf(s, 'right') },
      ]);
      // CM6 的 update 周期滞后于原生滚动, 接缝几何直接跟 scroll 事件逐帧走
      for (const v of views) v.scrollDOM.addEventListener('scroll', bumpTick, { passive: true });
      mounted = true;
      refreshDecos();
    } catch (e) {
      toast(String(e));
    }
  }

  /** 中栏扩展: 可编辑 + 历史 + 变更监听 */
  function centerExtensions(lang: Extension): Extension[] {
    return [
      lineNumbers(),
      history(),
      keymap.of([...defaultKeymap, ...historyKeymap]),
      ideaTheme,
      syntaxHighlighting(ideaHighlight, { fallback: true }),
      lang,
      geoTick,
      centerComp.of([]),
      EditorView.updateListener.of((u) => {
        if (u.docChanged) {
          const isOp = u.transactions.some((t) => t.annotation(chunkOp));
          if (!isOp) {
            // 手工编辑命中 chunk 区间 → 记为已手工解决(用旧坐标判定, 之后再重映射)
            u.changes.iterChangedRanges((fromA, toA) => {
              resultRanges.forEach((r, i) => {
                if (toA >= r.from && fromA <= r.to) states[i].edited = true;
              });
            });
          }
          for (const r of resultRanges) {
            r.from = u.changes.mapPos(r.from, -1);
            r.to = u.changes.mapPos(r.to, 1);
          }
          if (!isOp) scheduleDecos();
          scrollTick += 1;
        }
      }),
    ];
  }

  /** 该栏同步滚动锚点 */
  function anchorsOf(s: MergeSnapshot, pane: Pane): number[] {
    const key = pane === 'left' ? 'leftRange' : pane === 'right' ? 'rightRange' : 'resultRange';
    const text = pane === 'left' ? s.left : pane === 'right' ? s.right : s.result;
    const anchors = [0];
    for (const c of s.chunks) anchors.push(c[key][0], c[key][1]);
    anchors.push(text.split('\n').length);
    return anchors;
  }

  // ── chunk 状态与计数 ─────────────────────────────

  /** 该 chunk 需要处理的侧 */
  function relevantSides(c: MergeChunk): ('ours' | 'theirs')[] {
    if (c.kind === 'ours' || c.kind === 'agree') return ['ours'];
    if (c.kind === 'theirs') return ['theirs'];
    return ['ours', 'theirs'];
  }

  /** chunk 是否已解决(手工编辑或相关侧全部处理) */
  function resolved(id: number): boolean {
    const c = snap?.chunks[id];
    const st = states[id];
    if (!c || !st) return false;
    return st.edited || relevantSides(c).every((s) => st[s] !== 'pending');
  }

  // ── 装饰重建 ─────────────────────────────────────

  function leftClass(c: MergeChunk): string | null {
    if (c.kind === 'theirs') return null;
    const st = states[c.id];
    return st.edited || st.ours !== 'pending' ? 'ck-done' : `ck-${c.visual}`;
  }

  function rightClass(c: MergeChunk): string | null {
    if (c.kind === 'ours') return null;
    const st = states[c.id];
    return st.edited || st.theirs !== 'pending' ? 'ck-done' : `ck-${c.visual}`;
  }

  function centerClass(c: MergeChunk): string {
    const cls = resolved(c.id) ? 'ck-done' : `ck-${c.visual}`;
    return c.id === curIdx ? `${cls} ck-cur` : cls;
  }

  /** 重建三栏装饰(微任务合并, 避免在 update listener 内嵌套 dispatch) */
  function scheduleDecos() {
    if (decoScheduled) return;
    decoScheduled = true;
    queueMicrotask(() => {
      decoScheduled = false;
      refreshDecos();
    });
  }

  function refreshDecos() {
    if (!snap) return;
    const [lv, cv, rv] = views;
    if (!lv || !cv || !rv) return;
    const mk = (v: EditorView, pane: Pane, cls: (c: MergeChunk) => string | null) => {
      const [lines, marks, gutters] = buildPaneDecos(
        v.state.doc,
        snap!.chunks,
        pane,
        cls,
        pane === 'result' ? resultRanges : undefined
      );
      return [
        EditorView.decorations.of(lines),
        EditorView.decorations.of(marks),
        gutterLineClass.of(gutters),
      ];
    };
    lv.dispatch({ effects: leftComp.reconfigure(mk(lv, 'left', leftClass)) });
    cv.dispatch({ effects: centerComp.reconfigure(mk(cv, 'result', centerClass)) });
    rv.dispatch({ effects: rightComp.reconfigure(mk(rv, 'right', rightClass)) });
    scrollTick += 1;
  }

  // ── chunk 操作 ───────────────────────────────────

  /** 该侧在快照中的行内容 */
  function sideLines(c: MergeChunk, side: 'ours' | 'theirs'): string[] {
    return side === 'ours'
      ? leftLines.slice(c.leftRange[0], c.leftRange[1])
      : rightLines.slice(c.rightRange[0], c.rightRange[1]);
  }

  /** 记录撤销点(chunk 当前区间文本 + 状态) */
  function pushUndo(id: number) {
    const cv = views[1];
    const r = resultRanges[id];
    undoStack.push({ id, text: cv.state.doc.sliceString(r.from, r.to), state: { ...states[id] } });
    if (undoStack.length > 200) undoStack.shift();
  }

  /** 以 chunk 操作身份改写中栏(不进 CM6 历史) */
  function dispatchOp(from: number, to: number, insert: string) {
    views[1].dispatch({
      changes: { from, to, insert },
      annotations: [chunkOp.of(true), Transaction.addToHistory.of(false)],
    });
  }

  /** 区间替换文本组装: 维持行边界(区间尾在行首时补尾换行) */
  function joined(lines: string[], to: number): string {
    if (!lines.length) return '';
    const docLen = views[1].state.doc.length;
    return lines.join('\n') + (to < docLen ? '\n' : '');
  }

  /** 应用一侧到 Result; 已有一侧应用时追加(保留双方, 语义同 CLI 的 take order) */
  function applySide(id: number, side: 'ours' | 'theirs') {
    const c = snap?.chunks[id];
    const st = states[id];
    if (!c || !st || st[side] !== 'pending') return;
    pushUndo(id);
    const lines = sideLines(c, side);
    const r = resultRanges[id];
    const both = st.ours === 'applied' || st.theirs === 'applied';
    if (both) {
      const docLen = views[1].state.doc.length;
      let insert = joined(lines, r.to);
      if (insert && r.to >= docLen && docLen > 0) insert = '\n' + insert;
      dispatchOp(r.to, r.to, insert);
    } else {
      dispatchOp(r.from, r.to, joined(lines, r.to));
    }
    st[side] = 'applied';
    if (c.kind === 'agree') {
      st.ours = 'applied';
      st.theirs = 'applied';
    }
    scheduleDecos();
  }

  /** 忽略一侧(仅记账) */
  function ignoreSide(id: number, side: 'ours' | 'theirs') {
    const c = snap?.chunks[id];
    const st = states[id];
    if (!c || !st || st[side] !== 'pending') return;
    pushUndo(id);
    st[side] = 'ignored';
    if (c.kind === 'agree') {
      st.ours = 'ignored';
      st.theirs = 'ignored';
    }
    scheduleDecos();
  }

  /** 整 chunk 回到初始(base 内容 + 双侧 pending) */
  function revertChunk(id: number) {
    const c = snap?.chunks[id];
    if (!c) return;
    pushUndo(id);
    const r = resultRanges[id];
    const base = baseLines.slice(c.resultRange[0], c.resultRange[1]);
    dispatchOp(r.from, r.to, joined(base, r.to));
    states[id] = { ours: 'pending', theirs: 'pending', edited: false };
    scheduleDecos();
  }

  /** 全局撤销上一步 chunk 操作 */
  function undoLast() {
    const entry = undoStack.pop();
    if (!entry) return;
    const r = resultRanges[entry.id];
    dispatchOp(r.from, r.to, entry.text);
    states[entry.id] = entry.state;
    scheduleDecos();
  }

  /** 批量应用非冲突 chunk */
  function applyAll(direction: 'left' | 'all' | 'right') {
    if (!snap) return;
    for (const c of snap.chunks) {
      if (c.kind === 'conflict') continue;
      if (direction === 'left' && c.kind === 'theirs') continue;
      if (direction === 'right' && c.kind !== 'theirs') continue;
      const side = c.kind === 'theirs' ? 'theirs' : 'ours';
      if (states[c.id][side] === 'pending' && !states[c.id].edited) applySide(c.id, side);
    }
  }

  /** 上/下一个未解决 chunk(无未解决时在全部 chunk 间循环) */
  function nav(dir: 1 | -1) {
    if (!snap || !views[1]) return;
    const ids = snap.chunks.map((c) => c.id);
    const open = ids.filter((i) => !resolved(i));
    const pool = open.length ? open : ids;
    if (!pool.length) return;
    curIdx =
      dir > 0
        ? (pool.find((i) => i > curIdx) ?? pool[0])
        : ([...pool].reverse().find((i) => i < curIdx) ?? pool[pool.length - 1]);
    const r = resultRanges[curIdx];
    views[1].dispatch({ effects: EditorView.scrollIntoView(r.from, { y: 'center' }) });
    scheduleDecos();
  }

  // ── 落盘与离开 ───────────────────────────────────

  /** Apply: 中栏全文写入并暂存, 回列表; 仍有冲突时先弹应用内确认 */
  async function saveApply() {
    if (!snap || !views[1]) return;
    if (conflictsLeft > 0) {
      applyAsk = true;
      return;
    }
    await save(views[1].state.doc.toString());
  }

  /** 确认后无视剩余冲突强制 Apply */
  async function forceApply() {
    applyAsk = false;
    if (!snap || !views[1]) return;
    await save(views[1].state.doc.toString());
  }

  /** 整文件取一侧(Accept Left/Right) */
  async function acceptWhole(side: 'ours' | 'theirs') {
    if (!snap) return;
    await save(side === 'ours' ? snap.left : snap.right);
  }

  async function save(text: string) {
    if (!snap) return;
    try {
      await api.saveResult(snap.path, text);
      session.files = await api.conflicts();
      goto('/conflicts');
    } catch (e) {
      toast(String(e));
    }
  }

  // ── 接缝几何 ─────────────────────────────────────

  /** pos 在该 pane 视口内的 y 坐标; documentTop 已含滚动量与内容顶部内边距 */
  function posTop(v: EditorView, pos: number): number {
    return (
      v.lineBlockAt(Math.min(pos, v.state.doc.length)).top +
      v.documentTop -
      v.scrollDOM.getBoundingClientRect().top
    );
  }

  /** 行号(静态侧栏)对应的视口 y */
  function lineTopStatic(v: EditorView, line: number): number {
    const doc = v.state.doc;
    const pos = line < doc.lines ? doc.line(line + 1).from : doc.length;
    return posTop(v, pos);
  }

  /** 接缝内一个 chunk 的四角 y(侧栏起止 + 中栏起止); 对侧无关的 chunk 返回 null */
  function chunkGeom(c: MergeChunk, seam: 'l' | 'r') {
    void scrollTick;
    if (!mounted) return null;
    const side = seam === 'l' ? views[0] : views[2];
    const center = views[1];
    if (!side || !center) return null;
    if (seam === 'l' ? c.kind === 'theirs' : c.kind === 'ours') return null;
    const sr = seam === 'l' ? c.leftRange : c.rightRange;
    const ys1 = lineTopStatic(side, sr[0]);
    const ys2 = sr[1] > sr[0] ? lineTopStatic(side, sr[1]) : ys1 + 2;
    const r = resultRanges[c.id];
    const yc1 = posTop(center, r.from);
    const yc2 = r.to > r.from ? posTop(center, r.to) : yc1 + 2;
    return { ys1, ys2, yc1, yc2 };
  }

  /** 接缝连接带路径(左缝: 侧→中, 右缝镜像)。
      控制点放在两端 30% 处而非正中: 中段是匀坡近直线、只在进出口带圆角缓冲(IDEA 的斜扫观感),
      避免坡度全部堆在中段把窄带挤成竖管 */
  function bandPath(g: { ys1: number; ys2: number; yc1: number; yc2: number }, seam: 'l' | 'r') {
    const [x0, x1] = seam === 'l' ? [0, SEAM] : [SEAM, 0];
    const dx = (x1 - x0) * 0.3;
    return (
      `M${x0} ${g.ys1} C${x0 + dx} ${g.ys1} ${x1 - dx} ${g.yc1} ${x1} ${g.yc1}` +
      ` L${x1} ${g.yc2} C${x1 - dx} ${g.yc2} ${x0 + dx} ${g.ys2} ${x0} ${g.ys2} Z`
    );
  }

  /** 该 chunk 在此缝的待处理侧 */
  function seamSide(seam: 'l' | 'r'): 'ours' | 'theirs' {
    return seam === 'l' ? 'ours' : 'theirs';
  }

  /** 快捷键: F7/⇧F7 导航, ⌘/Ctrl+⏎ Apply, Esc 回列表(对齐 IDEA); 确认框打开时让位给对话框 */
  function hotkeys(e: KeyboardEvent) {
    if (applyAsk) return;
    if (e.key === 'F7') {
      e.preventDefault();
      nav(e.shiftKey ? -1 : 1);
    } else if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
      e.preventDefault();
      saveApply();
    } else if (e.key === 'Escape') {
      goto('/conflicts');
    }
  }

  /** ruler 点击跳转 */
  function jump(e: MouseEvent) {
    const result = views[1];
    if (!result || !snap) return;
    const el = e.currentTarget as HTMLElement;
    const frac = (e.clientY - el.getBoundingClientRect().top) / el.clientHeight;
    const center = frac * totalResultLines * result.defaultLineHeight;
    result.scrollDOM.scrollTop = Math.max(0, center - result.scrollDOM.clientHeight / 2);
  }
</script>

<svelte:window onkeydown={hotkeys} />

<div class="merge" class:no-em={!showWords}>
  <header>
    <button onclick={() => goto('/conflicts')}>← Conflicts</button>
    <span class="title mono">{path}</span>
    <span class="tsep"></span>
    <button class="tb" title="Previous change" onclick={() => nav(-1)}>↑</button>
    <button class="tb" title="Next change" onclick={() => nav(1)}>↓</button>
    <span class="tsep"></span>
    <span class="tlabel dim">Apply non-conflicting:</span>
    <button class="tb" title="Apply all from left" onclick={() => applyAll('left')}>≫ Left</button>
    <button class="tb" title="Apply all non-conflicting" onclick={() => applyAll('all')}>⋙ All</button>
    <button class="tb" title="Apply all from right" onclick={() => applyAll('right')}>≪ Right</button>
    <span class="tsep"></span>
    <button
      class="tb"
      class:on={showWords}
      title="Toggle word-level highlight"
      onclick={() => (showWords = !showWords)}>Highlight words</button
    >
    <span class="spacer"></span>
    {#if snap}
      <span class="counts" class:ok={changesLeft === 0}>
        {#if changesLeft === 0}All changes processed.{:else}
          {changesLeft} change{changesLeft === 1 ? '' : 's'}. {conflictsLeft} conflict{conflictsLeft === 1 ? '' : 's'}.
        {/if}
      </span>
    {/if}
  </header>

  {#if snap}
    <div class="labels">
      <span class="pane-label">Changes from <b>{session.info?.yoursLabel}</b></span>
      <span class="pane-label center">Result</span>
      <span class="pane-label">Changes from <b>{session.info?.theirsLabel}</b></span>
    </div>

    <div class="panes">
      <div class="pane" bind:this={leftEl}></div>

      <div class="seam" onwheel={seamWheel}>
        <svg class="band" width={SEAM} height="100%">
          {#each snap.chunks as c (c.id)}
            {@const g = chunkGeom(c, 'l')}
            {#if g}
              <path
                d={bandPath(g, 'l')}
                class="bandp"
                class:done={resolved(c.id)}
                style="--band-color:var(--chunk-{c.visual})"
              />
            {/if}
          {/each}
        </svg>
        {#each snap.chunks as c (c.id)}
          {@const g = chunkGeom(c, 'l')}
          {#if g}
            <div class="gbtns" style="top:{g.ys1 + 1}px">
              {#if !states[c.id].edited && states[c.id][seamSide('l')] === 'pending'}
                <button class="gb" title="Ignore" onclick={() => ignoreSide(c.id, 'ours')}>{@html IC.ignore}</button>
                <button class="gb" title="Apply this change" onclick={() => applySide(c.id, 'ours')}>{@html IC.applyL}</button>
              {:else}
                <button class="gb" title="Revert this chunk" onclick={() => revertChunk(c.id)}>{@html IC.revert}</button>
              {/if}
            </div>
          {/if}
        {/each}
      </div>

      <div class="pane center" bind:this={resultEl}></div>

      <div class="seam" onwheel={seamWheel}>
        <svg class="band" width={SEAM} height="100%">
          {#each snap.chunks as c (c.id)}
            {@const g = chunkGeom(c, 'r')}
            {#if g}
              <path
                d={bandPath(g, 'r')}
                class="bandp"
                class:done={resolved(c.id)}
                style="--band-color:var(--chunk-{c.visual})"
              />
            {/if}
          {/each}
        </svg>
        {#each snap.chunks as c (c.id)}
          {@const g = chunkGeom(c, 'r')}
          {#if g}
            <div class="gbtns right" style="top:{g.ys1 + 1}px">
              {#if !states[c.id].edited && states[c.id][seamSide('r')] === 'pending'}
                <button class="gb" title="Apply this change" onclick={() => applySide(c.id, 'theirs')}>{@html IC.applyR}</button>
                <button class="gb" title="Ignore" onclick={() => ignoreSide(c.id, 'theirs')}>{@html IC.ignore}</button>
              {:else}
                <button class="gb" title="Revert this chunk" onclick={() => revertChunk(c.id)}>{@html IC.revert}</button>
              {/if}
            </div>
          {/if}
        {/each}
      </div>

      <div class="pane" bind:this={rightEl}></div>

      <div class="ruler" onclick={jump} role="presentation">
        {#each snap.chunks as c (c.id)}
          <span
            class="rb ck-{resolved(c.id) ? 'done' : c.visual}"
            style="top:{(c.resultRange[0] / Math.max(1, totalResultLines)) * 100}%;height:{Math.max(
              0.5,
              ((c.resultRange[1] - c.resultRange[0]) / Math.max(1, totalResultLines)) * 100
            )}%"
          ></span>
        {/each}
      </div>
    </div>

    <footer>
      <button onclick={() => acceptWhole('ours')}>Accept Left</button>
      <button onclick={() => acceptWhole('theirs')}>Accept Right</button>
      <button disabled={!undoStack.length} onclick={undoLast}>Undo</button>
      <span class="spacer"></span>
      <button onclick={() => goto('/conflicts')}>Cancel</button>
      <button class="primary" onclick={saveApply}>Apply</button>
    </footer>
  {:else}
    <div class="loading dim">Loading…</div>
  {/if}
</div>

{#if applyAsk}
  <ConfirmDialog
    title="Apply"
    message={`${conflictsLeft} conflict${conflictsLeft === 1 ? '' : 's'} still unresolved. Apply anyway?`}
    confirmLabel="Apply"
    onconfirm={forceApply}
    onclose={() => (applyAsk = false)}
  />
{/if}

<style>
  .merge {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--d-canvas);
  }

  header,
  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--d-panel);
  }

  header {
    border-bottom: 1px solid var(--d-border);
  }

  footer {
    border-top: 1px solid var(--d-border);
  }

  .title {
    font-size: 12px;
  }

  .tsep {
    width: 1px;
    height: 16px;
    background: var(--d-border-strong);
    margin: 0 4px;
  }

  .tb {
    padding: 0 8px;
  }

  .tb.on {
    background: #243252;
    border-color: #3a4c74;
    color: var(--d-sel-text);
  }

  .merge.no-em :global(.ck-em) {
    background: transparent;
    border-radius: 0;
  }

  .tlabel {
    font-size: 11px;
  }

  .counts {
    font-size: 12px;
    color: var(--d-dim);
  }

  .counts.ok {
    color: var(--d-green);
  }

  .spacer {
    flex: 1;
  }

  .labels {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    border-bottom: 1px solid var(--d-border);
    background: var(--d-panel);
  }

  .pane-label {
    padding: 5px 12px;
    font-size: 11px;
    color: var(--d-dim);
    border-right: 1px solid var(--d-border);
  }

  .pane-label b {
    color: var(--d-text);
    font-weight: 600;
  }

  .pane-label.center {
    text-align: center;
  }

  .panes {
    flex: 1;
    display: flex;
    min-height: 0;
    position: relative;
    padding-right: 12px;
  }

  .pane {
    flex: 1;
    min-width: 0;
    overflow: hidden;
  }

  /* 接缝不设竖边框: 色带与两侧行底色齐平衔接, 读作一个连续形状(IDEA 行为)。
     宽度与脚本里的 SEAM 常量保持一致 */
  .seam {
    width: 44px;
    flex: none;
    position: relative;
    overflow: hidden;
    background: var(--d-canvas);
  }

  .band {
    position: absolute;
    inset: 0;
    height: 100%;
    pointer-events: none;
  }

  /* 连接带: 实心同 chunk 底色、无描边 → 与行底色连成漏斗形; 已解决降为极淡 */
  .bandp {
    fill: var(--band-color);
  }

  .bandp.done {
    fill: rgba(255, 255, 255, 0.045);
  }

  .gbtns {
    position: absolute;
    left: 0;
    right: 0;
    display: flex;
    justify-content: center;
    gap: 2px;
  }

  /* IDEA 风格: 无底色细线 glyph, 直接浮在连接带上, hover 才显浅色圆角底 */
  .gb {
    width: 15px;
    height: 15px;
    padding: 0;
    border: none;
    border-radius: 3px;
    background: transparent;
    color: var(--d-dim);
    display: grid;
    place-items: center;
  }

  .gb:hover {
    background: var(--d-hover);
    color: var(--d-text);
  }

  .ruler {
    position: absolute;
    top: 0;
    right: 0;
    width: 12px;
    height: 100%;
    background: var(--d-panel);
    border-left: 1px solid var(--d-border);
    cursor: pointer;
  }

  .rb {
    position: absolute;
    left: 2px;
    right: 2px;
    min-height: 3px;
    border-radius: 1px;
  }

  .loading {
    flex: 1;
    display: grid;
    place-items: center;
  }

  /* chunk 装饰(CM6 内容 / 接缝 / ruler 共用色) */
  :global(.ck-modified) {
    background: var(--chunk-modified);
  }

  :global(.ck-added) {
    background: var(--chunk-added);
  }

  :global(.ck-deleted) {
    background: var(--chunk-deleted);
  }

  :global(.ck-conflict) {
    background: var(--chunk-conflict);
  }

  :global(.ck-done) {
    background: rgba(255, 255, 255, 0.045);
  }

  :global(.ck-cur) {
    outline: 1px solid rgba(138, 180, 255, 0.55);
    outline-offset: -1px;
  }

  :global(.ck-em) {
    background: rgba(255, 255, 255, 0.16);
    border-radius: 2px;
  }
</style>
