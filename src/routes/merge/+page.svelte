<script lang="ts">
  // 三栏合并页(M3: 完整交互): gutter 应用/忽略、接缝连接带、撤销、批量应用、
  // 导航、中栏自由编辑(命中 chunk 记为手工解决)、Apply 落盘
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { Annotation, Compartment, Transaction, type Extension } from '@codemirror/state';
  import { EditorView, gutterLineClass, keymap, lineNumbers } from '@codemirror/view';
  import { history, historyKeymap, defaultKeymap } from '@codemirror/commands';
  import { api, type MergeChunk, type MergeSnapshot } from '$lib/api';
  import {
    applyAllTargets,
    applyEdit,
    clipCovers,
    isResolved,
    joinedText,
    navTarget,
    paddedClip,
    paneClass,
    type ChunkState,
  } from '$lib/chunks';
  import { session } from '$lib/state.svelte';
  import { settings, settingsUi } from '$lib/settings.svelte';
  import { toast } from '$lib/toast.svelte';
  import { largeWindow } from '$lib/win';
  import ConfirmDialog from '$lib/components/ConfirmDialog.svelte';
  import {
    appearanceExtensions,
    buildPaneDecos,
    createPane,
    languageFor,
    lineRangeToPos,
    linkScroll,
    readonlyExtensions,
    type Pane,
  } from '$lib/editor';

  const path = $derived(page.url.searchParams.get('path') ?? '');

  interface UndoEntry {
    id: number;
    text: string;
    state: ChunkState;
  }

  const SEAM = 44;
  // 装饰裁剪窗口: 视口外扩 400 行构建, 视口逼近窗口边缘 100 行内才重建——
  // 平滑滚动时每滚约 300 行才有一次装饰 dispatch, 其余帧零重建
  const PAD_LINES = 400;
  const GUARD_LINES = 100;
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
  // 初值取设置的默认开关; 页内 toggle 只影响本次会话, 不写回设置
  let showWords = $state(settings.value.highlightWords);
  let applyAsk = $state(false);
  let states = $state<ChunkState[]>([]);
  let resultRanges = $state<{ from: number; to: number }[]>([]);
  let undoStack = $state<UndoEntry[]>([]);
  let curIdx = $state(-1);
  let scrollTick = $state(0);
  let docTick = $state(0);
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
  // 每栏装饰缓存: 类名签名 + 已建裁剪窗口。签名与窗口都未失效(手工打字/小幅滚动的
  // 高频路径)就跳过该栏重建; 中栏区间随编辑漂移, 每次调度都重建(有裁剪, 成本有界)
  const decoCache: { sig: string; clip: { from: number; to: number } | null }[] = [
    { sig: '', clip: null },
    { sig: '', clip: null },
    { sig: '', clip: null },
  ];

  const changesLeft = $derived.by(() => states.filter((_, i) => !resolved(i)).length);
  const conflictsLeft = $derived.by(() =>
    snap ? snap.chunks.filter((c) => c.kind === 'conflict' && !resolved(c.id)).length : 0
  );
  interface SeamGeom {
    ys1: number;
    ys2: number;
    yc1: number;
    yc2: number;
  }

  /// 接缝几何统一在此每帧计算一次: 视口粗筛(中栏实时区间 × CM6 viewport 自带外扩)后,
  /// 为可见 chunk 算出左右缝四角 y。坐标基准(documentTop / getBoundingClientRect)
  /// 每帧每栏只读一次——布局读取集中在模板写入之前, 消除逐 chunk 交错读写的强制布局;
  /// SVG 路径与按钮两个 each 块共享同一份结果, 几何不再算两遍
  const seamGeoms = $derived.by(() => {
    void scrollTick;
    if (!mounted || !snap) return [];
    const [lv, cv, rv] = views;
    if (!lv || !cv || !rv) return [];
    const vp = cv.viewport;
    const base = [lv, cv, rv].map((v) => v.documentTop - v.scrollDOM.getBoundingClientRect().top);
    // pos 在该 pane 视口内的 y(基准已含滚动量与内容顶部内边距)
    const topAt = (v: EditorView, b: number, pos: number) =>
      v.lineBlockAt(Math.min(pos, v.state.doc.length)).top + b;
    // 行号(静态侧栏)对应的视口 y
    const lineTop = (v: EditorView, b: number, line: number) => {
      const doc = v.state.doc;
      return topAt(v, b, line < doc.lines ? doc.line(line + 1).from : doc.length);
    };
    const side = (v: EditorView, b: number, sr: [number, number], yc1: number, yc2: number) => {
      const ys1 = lineTop(v, b, sr[0]);
      const ys2 = sr[1] > sr[0] ? lineTop(v, b, sr[1]) : ys1 + 2;
      return { ys1, ys2, yc1, yc2 };
    };
    const out: { c: MergeChunk; l: SeamGeom | null; r: SeamGeom | null }[] = [];
    for (const c of snap.chunks) {
      const rr = resultRanges[c.id];
      if (!rr || rr.to < vp.from || rr.from > vp.to) continue;
      const yc1 = topAt(cv, base[1], rr.from);
      const yc2 = rr.to > rr.from ? topAt(cv, base[1], rr.to) : yc1 + 2;
      out.push({
        c,
        l: c.kind === 'theirs' ? null : side(lv, base[0], c.leftRange, yc1, yc2),
        r: c.kind === 'ours' ? null : side(rv, base[2], c.rightRange, yc1, yc2),
      });
    }
    return out;
  });

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
  /// 否则按初始估算行高算出的按钮/色带位置会随行号累积偏移;
  /// 视口移动时检查装饰裁剪窗口是否将出界, 是则调度重建(滞回, 并非逐帧)
  const geoTick = EditorView.updateListener.of((u) => {
    if (u.geometryChanged) bumpTick();
    if (u.viewportChanged && clipsStale()) scheduleDecos();
  });

  /** 任一栏视口逼近其装饰裁剪窗口边缘 → 需要重建 */
  function clipsStale(): boolean {
    if (!mounted) return false;
    return views.some(
      (v, i) => !clipCovers(v.state.doc, decoCache[i].clip, v.viewport, GUARD_LINES)
    );
  }

  /** 拉取快照并构建三栏 */
  async function load() {
    try {
      // 快照(Rust 引擎)与语言包(动态 import)无依赖, 并行拉取
      const [s, lang] = await Promise.all([api.openMerge(path), languageFor(path)]);
      snap = s;
      leftLines = s.left.split('\n');
      rightLines = s.right.split('\n');
      baseLines = s.result.split('\n');
      states = s.chunks.map(() => ({ ours: 'pending', theirs: 'pending', edited: false }));
      await new Promise((r) => requestAnimationFrame(r));
      if (!leftEl || !resultEl || !rightEl) return;

      // 编辑器随路由进入新建, 亮暗按当次设置选用(改主题后重进三栏生效)
      const light = settings.value.theme === 'light';
      const left = createPane(leftEl, s.left, [
        ...readonlyExtensions(lang, light),
        geoTick,
        leftComp.of([]),
      ]);
      const right = createPane(rightEl, s.right, [
        ...readonlyExtensions(lang, light),
        geoTick,
        rightComp.of([]),
      ]);
      const result = createPane(resultEl, s.result, centerExtensions(lang, light));
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
  function centerExtensions(lang: Extension, light: boolean): Extension[] {
    return [
      lineNumbers(),
      history(),
      keymap.of([...defaultKeymap, ...historyKeymap]),
      ...appearanceExtensions(light),
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
          docTick += 1;
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

  // ── chunk 状态与计数(纯逻辑在 $lib/chunks, 此处按 id 取上下文) ──

  /** chunk 是否已解决(手工编辑或相关侧全部处理) */
  function resolved(id: number): boolean {
    return isResolved(snap?.chunks[id], states[id]);
  }

  // ── 装饰重建 ─────────────────────────────────────

  const leftClass = (c: MergeChunk) => paneClass(c, states[c.id], 'left');
  const rightClass = (c: MergeChunk) => paneClass(c, states[c.id], 'right');

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
    // 每栏只为裁剪窗口内的行构建装饰(成本与视口成正比, 与文件/chunk 大小无关);
    // 侧栏文档只读不变: 类名签名与裁剪窗口都未失效就跳过该栏重建
    // (手工打字只触发中栏重建, 侧栏的词级强调不逐键重算);
    // 中栏区间随编辑漂移无廉价签名, force 每次调度都重建
    const upd = (
      idx: number,
      v: EditorView,
      pane: Pane,
      cls: (c: MergeChunk) => string | null,
      comp: Compartment,
      force: boolean
    ) => {
      const sig = snap!.chunks.map(cls).join(' ');
      const cache = decoCache[idx];
      if (
        !force &&
        sig === cache.sig &&
        clipCovers(v.state.doc, cache.clip, v.viewport, GUARD_LINES)
      ) {
        return;
      }
      const clip = paddedClip(v.state.doc, v.viewport, PAD_LINES);
      cache.sig = sig;
      cache.clip = clip;
      const [lines, marks, gutters] = buildPaneDecos(
        v.state.doc,
        snap!.chunks,
        pane,
        cls,
        pane === 'result' ? resultRanges : undefined,
        clip
      );
      v.dispatch({
        effects: comp.reconfigure([
          EditorView.decorations.of(lines),
          EditorView.decorations.of(marks),
          gutterLineClass.of(gutters),
        ]),
      });
    };
    upd(0, lv, 'left', leftClass, leftComp, false);
    upd(2, rv, 'right', rightClass, rightComp, false);
    upd(1, cv, 'result', centerClass, centerComp, true);
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

  /** 应用一侧到 Result; 已有一侧应用时追加(保留双方, 语义同 CLI 的 take order) */
  function applySide(id: number, side: 'ours' | 'theirs') {
    const c = snap?.chunks[id];
    const st = states[id];
    if (!c || !st || st[side] !== 'pending') return;
    pushUndo(id);
    const both = st.ours === 'applied' || st.theirs === 'applied';
    const e = applyEdit(sideLines(c, side), resultRanges[id], both, views[1].state.doc.length);
    dispatchOp(e.from, e.to, e.insert);
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
    dispatchOp(r.from, r.to, joinedText(base, r.to, views[1].state.doc.length));
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

  /** 批量应用非冲突 chunk; agree 双方内容一致, 两个方向都放行(applySide 会同时记账双侧) */
  function applyAll(direction: 'left' | 'all' | 'right') {
    if (!snap) return;
    for (const t of applyAllTargets(snap.chunks, states, direction)) applySide(t.id, t.side);
  }

  /** 上/下一个未解决 chunk(无未解决时在全部 chunk 间循环) */
  function nav(dir: 1 | -1) {
    if (!snap || !views[1]) return;
    const ids = snap.chunks.map((c) => c.id);
    const open = ids.filter((i) => !resolved(i));
    const pool = open.length ? open : ids;
    if (!pool.length) return;
    curIdx = navTarget(pool, curIdx, dir);
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
  // (四角 y 的计算集中在 seamGeoms derived, 每帧一次)

  /** 接缝连接带路径(左缝: 侧→中, 右缝镜像)。
      控制点放在两端 30% 处而非正中: 中段是匀坡近直线、只在进出口带圆角缓冲(IDEA 的斜扫观感),
      避免坡度全部堆在中段把窄带挤成竖管 */
  function bandPath(g: SeamGeom, seam: 'l' | 'r') {
    const [x0, x1] = seam === 'l' ? [0, SEAM] : [SEAM, 0];
    const dx = (x1 - x0) * 0.3;
    return (
      `M${x0} ${g.ys1} C${x0 + dx} ${g.ys1} ${x1 - dx} ${g.yc1} ${x1} ${g.yc1}` +
      ` L${x1} ${g.yc2} C${x1 - dx} ${g.yc2} ${x0 + dx} ${g.ys2} ${x0} ${g.ys2} Z`
    );
  }

  /** 快捷键: F7/⇧F7 导航, ⌘/Ctrl+⏎ Apply, Esc 回列表(对齐 IDEA); 对话框打开时让位 */
  function hotkeys(e: KeyboardEvent) {
    if (applyAsk || settingsUi.open) return;
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

  /** ruler 色块几何(百分比): 用中栏实时区间与实时行数换算, 编辑后不漂移 */
  function rulerBlock(c: MergeChunk): { top: number; h: number } | null {
    void docTick;
    const v = views[1];
    const r = resultRanges[c.id];
    if (!mounted || !v || !r) return null;
    const doc = v.state.doc;
    const start = doc.lineAt(Math.min(r.from, doc.length)).number - 1;
    const end = r.to > r.from ? doc.lineAt(Math.min(r.to, doc.length)).number - 1 : start;
    return {
      top: (start / Math.max(1, doc.lines)) * 100,
      h: Math.max(0.5, ((end - start) / Math.max(1, doc.lines)) * 100),
    };
  }

  /** ruler 点击跳转: 按实际滚动高度换算(含内边距与实测行高, 编辑后不漂移) */
  function jump(e: MouseEvent) {
    const result = views[1];
    if (!result) return;
    const el = e.currentTarget as HTMLElement;
    const frac = (e.clientY - el.getBoundingClientRect().top) / el.clientHeight;
    const sd = result.scrollDOM;
    sd.scrollTop = Math.max(0, frac * sd.scrollHeight - sd.clientHeight / 2);
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
          {#each seamGeoms as sg (sg.c.id)}
            {#if sg.l}
              <path
                d={bandPath(sg.l, 'l')}
                class="bandp"
                class:done={resolved(sg.c.id)}
                style="--band-color:var(--chunk-{sg.c.visual})"
              />
            {/if}
          {/each}
        </svg>
        {#each seamGeoms as sg (sg.c.id)}
          {#if sg.l}
            <div class="gbtns" style="top:{sg.l.ys1 + 1}px">
              {#if !states[sg.c.id].edited && states[sg.c.id].ours === 'pending'}
                <button class="gb" title="Ignore" onclick={() => ignoreSide(sg.c.id, 'ours')}>{@html IC.ignore}</button>
                <button class="gb" title="Apply this change" onclick={() => applySide(sg.c.id, 'ours')}>{@html IC.applyL}</button>
              {:else}
                <button class="gb" title="Revert this chunk" onclick={() => revertChunk(sg.c.id)}>{@html IC.revert}</button>
              {/if}
            </div>
          {/if}
        {/each}
      </div>

      <div class="pane center" bind:this={resultEl}></div>

      <div class="seam" onwheel={seamWheel}>
        <svg class="band" width={SEAM} height="100%">
          {#each seamGeoms as sg (sg.c.id)}
            {#if sg.r}
              <path
                d={bandPath(sg.r, 'r')}
                class="bandp"
                class:done={resolved(sg.c.id)}
                style="--band-color:var(--chunk-{sg.c.visual})"
              />
            {/if}
          {/each}
        </svg>
        {#each seamGeoms as sg (sg.c.id)}
          {#if sg.r}
            <div class="gbtns right" style="top:{sg.r.ys1 + 1}px">
              {#if !states[sg.c.id].edited && states[sg.c.id].theirs === 'pending'}
                <button class="gb" title="Apply this change" onclick={() => applySide(sg.c.id, 'theirs')}>{@html IC.applyR}</button>
                <button class="gb" title="Ignore" onclick={() => ignoreSide(sg.c.id, 'theirs')}>{@html IC.ignore}</button>
              {:else}
                <button class="gb" title="Revert this chunk" onclick={() => revertChunk(sg.c.id)}>{@html IC.revert}</button>
              {/if}
            </div>
          {/if}
        {/each}
      </div>

      <div class="pane" bind:this={rightEl}></div>

      <div class="ruler" onclick={jump} role="presentation">
        {#each snap.chunks as c (c.id)}
          {@const rb = rulerBlock(c)}
          {#if rb}
            <span
              class="rb ck-{resolved(c.id) ? 'done' : c.visual}"
              style="top:{rb.top}%;height:{rb.h}%"
            ></span>
          {/if}
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
    background: var(--d-sel-dim);
    border-color: var(--d-sel-dim-border);
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
    /* 不加 CSS containment: WKWebView 对 contain:paint + 滚动容器(含 sticky gutter)
       有重绘 bug——滚动后接缝色带留下鬼影且新帧不再绘制 */
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
    fill: var(--chunk-done);
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
    background: var(--chunk-done);
  }

  :global(.ck-cur) {
    outline: 1px solid var(--ck-cur-outline);
    outline-offset: -1px;
  }

  :global(.ck-em) {
    background: var(--chunk-em);
    border-radius: 2px;
  }
</style>
