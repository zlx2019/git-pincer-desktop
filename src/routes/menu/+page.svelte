<script lang="ts">
  // 菜单指令面板(IDEA New UI 暗色小窗, 见 docs/IDEA_STYLE.md 与根目录 mockup):
  // 五操作对齐 CLI, ⌘1–⌘5 快捷键, 底部终端风执行输出; 出现冲突立刻接管切大窗
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { api, type LaunchKind, type OutputLine } from '$lib/api';
  import { session, term } from '$lib/state.svelte';
  import { toast } from '$lib/toast.svelte';
  import { compactWindow } from '$lib/win';
  import PickerDialog from '$lib/components/PickerDialog.svelte';

  interface DialogSpec {
    kind: LaunchKind;
    title: string;
    multi: boolean;
    confirm: string;
    items: { id: string; label: string; sublabel?: string; disabled?: boolean }[];
    order: string[];
  }

  let running: LaunchKind | null = $state(null);
  let dialog: DialogSpec | null = $state(null);
  let switchItems: { id: string; label: string; sublabel?: string; disabled?: boolean }[] | null =
    $state(null);
  let termEl: HTMLElement | undefined = $state();

  // 图标为静态字面量, {@html} 安全
  const actions: { kind: LaunchKind; label: string; zh: string; desc: string; icon: string }[] = [
    {
      kind: 'pull',
      label: 'pull',
      zh: '拉取远端',
      desc: '从跟踪的远端拉取最新提交并合并',
      icon: '<svg viewBox="0 0 16 16" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M8 2.5v7M4.8 6.8 8 10l3.2-3.2M3 13.5h10"/></svg>',
    },
    {
      kind: 'merge',
      label: 'merge',
      zh: '合并分支',
      desc: '选择一个分支合并进当前分支',
      icon: '<svg viewBox="0 0 16 16" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="4.5" cy="3.5" r="1.7"/><circle cx="4.5" cy="12.5" r="1.7"/><circle cx="11.5" cy="8" r="1.7"/><path d="M4.5 5.2v5.6M4.5 6.5c0 2 2.7 1.5 5.3 1.5"/></svg>',
    },
    {
      kind: 'rebase',
      label: 'rebase',
      zh: '变基分支',
      desc: '将当前分支变基到目标分支之上',
      icon: '<svg viewBox="0 0 16 16" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M3 4h6.5M3 8h4.5M3 12h6.5M12.5 3.5v7.5M10.6 9.2l1.9 1.9 1.9-1.9"/></svg>',
    },
    {
      kind: 'cherry-pick',
      label: 'cherry-pick',
      zh: '摘取提交',
      desc: '从其他分支摘取提交应用到当前分支',
      icon: '<svg viewBox="0 0 16 16" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="5.4" cy="11.2" r="2.1"/><circle cx="10.9" cy="11.9" r="2.1"/><path d="M5.4 9.1C5.4 6 7.6 4.6 10.2 2.6M10.9 9.8C10.6 6.9 9.7 5.4 10.2 2.6"/></svg>',
    },
    {
      kind: 'revert',
      label: 'revert',
      zh: '撤销提交',
      desc: '生成反向提交, 撤销所选提交的改动',
      icon: '<svg viewBox="0 0 16 16" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M3.5 3.5v4h4"/><path d="M3.8 7.2a4.9 4.9 0 1 0 1.1-3"/></svg>',
    },
  ];

  onMount(() => {
    compactWindow().catch(() => {});
    if (!session.info) {
      goto('/');
      return;
    }
    // 操作进行中且未被用户搁置 → 接管进冲突页; 搁置时留在菜单(顶部有恢复横幅)
    if (session.info.op && !session.parked) {
      goto('/conflicts');
      return;
    }
    // 终端里发起的操作也要能接管: 聚焦时重探
    let unlisten: (() => void) | undefined;
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused) reprobe();
      })
      .then((u) => (unlisten = u));
    return () => unlisten?.();
  });

  /** 聚焦重探: 有操作进行中且未搁置则接管进冲突页; 搁置期间只刷新横幅数据 */
  async function reprobe() {
    if (running || !session.info) return;
    try {
      const info = await api.repoOpen();
      session.info = info;
      if (!info.op) {
        // 操作已结束(在终端完成/中止): 搁置随之失效
        session.parked = false;
        return;
      }
      session.files = await api.conflicts();
      if (!session.parked) goto('/conflicts');
    } catch (e) {
      toast(String(e));
    }
  }

  /** 恢复被搁置的冲突解决 */
  function resume() {
    session.parked = false;
    goto('/conflicts');
  }

  /** 打开分支切换对话框(当前分支置顶且置灰) */
  async function askSwitch() {
    if (running || dialog) return;
    try {
      const bs = (await api.branches()).sort((a, b) => Number(b.current) - Number(a.current));
      switchItems = bs.map((b) => ({
        id: b.name,
        label: b.name,
        sublabel: b.current ? 'current' : undefined,
        disabled: b.current,
      }));
    } catch (e) {
      toast(String(e));
    }
  }

  /** 执行切换: 回执进终端, 失败原样透出 git 的说明(如工作区有未提交改动) */
  async function doSwitch(ids: string[]) {
    const name = ids[0];
    switchItems = null;
    term.entries.push({ kind: 'cmd', text: `git switch ${name}` });
    try {
      await api.switchBranch(name);
      session.info = await api.repoOpen();
      term.entries.push({ kind: 'ok', text: `✔ switched to ${name}` });
    } catch (e) {
      term.entries.push({ kind: 'fail', text: `✘ ${String(e)}` });
    }
  }

  /** ⌘/Ctrl + 1..5 触发对应操作 */
  function hotkey(e: KeyboardEvent) {
    if (!(e.metaKey || e.ctrlKey) || dialog || switchItems || running) return;
    const idx = ['1', '2', '3', '4', '5'].indexOf(e.key);
    if (idx >= 0) {
      e.preventDefault();
      act(actions[idx].kind);
    }
  }

  /** 点操作: pull 直接执行, merge/rebase 弹分支选择, cherry-pick/revert 弹提交选择 */
  async function act(kind: LaunchKind) {
    // 已有操作进行中(搁置状态)时不允许再发起, git 也会拒绝
    if (running || session.info?.op) return;
    const current = session.info?.yoursLabel ?? '';
    try {
      if (kind === 'pull') {
        await run(kind, []);
      } else if (kind === 'merge' || kind === 'rebase') {
        // 当前分支置顶(IDEA 习惯), 其余保持 git 的字母序
        const bs = (await api.branches()).sort((a, b) => Number(b.current) - Number(a.current));
        dialog = {
          kind,
          multi: false,
          confirm: kind === 'merge' ? 'Merge' : 'Rebase',
          title: kind === 'merge' ? `合并分支 → ${current}` : `变基 ${current} 到目标分支`,
          items: bs.map((b) => ({
            id: b.name,
            label: b.name,
            sublabel: b.current ? 'current' : undefined,
            disabled: b.current,
          })),
          order: bs.map((b) => b.name),
        };
      } else {
        const cs = await api.commits(kind === 'cherry-pick', 30);
        dialog = {
          kind,
          multi: true,
          confirm: kind === 'cherry-pick' ? 'Cherry-pick' : 'Revert',
          title: kind === 'cherry-pick' ? '摘取提交(可多选)' : '撤销提交(可多选)',
          items: cs.map((c) => ({
            id: c.sha,
            label: c.subject,
            sublabel: c.sha,
            tag: c.branch || undefined,
          })),
          order: cs.map((c) => c.sha),
        };
      }
    } catch (e) {
      toast(String(e));
    }
  }

  /** 对话框确认: 列表为新→旧, cherry-pick 需按旧→新逐个应用, revert 保持新→旧 */
  function confirmDialog(ids: string[]) {
    if (!dialog) return;
    const order = dialog.order;
    const byOrder = [...ids].sort((a, b) => order.indexOf(a) - order.indexOf(b));
    const targets = dialog.kind === 'cherry-pick' ? byOrder.reverse() : byOrder;
    const kind = dialog.kind;
    dialog = null;
    run(kind, targets);
  }

  /** 执行操作: 命令回显 + 输出流入终端; 出冲突 → 大窗冲突页, 结束打尾行 */
  async function run(kind: LaunchKind, targets: string[]) {
    running = kind;
    term.entries.push({ kind: 'cmd', text: ['git', kind, ...targets].join(' ') });
    const started = Date.now();
    const unlisten = await api.onOutput((l: OutputLine) =>
      term.entries.push({ kind: l.stream === 'stderr' ? 'err' : 'out', text: l.line })
    );
    try {
      const outcome = await api.launchOp(kind, targets);
      session.info = await api.repoOpen();
      if (outcome.kind === 'conflicts') {
        session.files = outcome.files;
        await goto('/conflicts');
      } else if (outcome.kind === 'cleanDone') {
        const secs = ((Date.now() - started) / 1000).toFixed(1);
        term.entries.push({ kind: 'ok', text: `✔ 完成 · 用时 ${secs}s` });
      } else {
        term.entries.push({ kind: 'fail', text: `✘ ${outcome.message}` });
      }
    } catch (e) {
      term.entries.push({ kind: 'fail', text: `✘ ${String(e)}` });
    } finally {
      unlisten();
      running = null;
    }
  }

  // 终端自动滚到底
  $effect(() => {
    void term.entries.length;
    if (termEl) termEl.scrollTop = termEl.scrollHeight;
  });

  /** 路径末段作为展示名 */
  function basename(p: string): string {
    return p.split('/').filter(Boolean).pop() ?? p;
  }
</script>

<svelte:window onkeydown={hotkey} />

{#if session.info}
  {@const info = session.info}
  <div class="win">
    <div class="topbar">
      <span class="chip" title={info.root}>
        <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
          <path d="M1.5 4.5a1 1 0 0 1 1-1h3l1.5 1.8h6.5a1 1 0 0 1 1 1v6.2a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z" />
        </svg>
        {basename(info.root)}
      </span>
      <button
        class="chip"
        title="Switch branch"
        disabled={running !== null || !!info.op}
        onclick={askSwitch}
      >
        <svg class="bicon" viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
          <circle cx="4.5" cy="3.5" r="1.6" /><circle cx="4.5" cy="12.5" r="1.6" /><circle cx="11.5" cy="6" r="1.6" />
          <path d="M4.5 5.1v5.8M11.5 7.6c0 2.4-4 1.6-6 2.6" />
        </svg>
        <span class="mono">{info.yoursLabel}</span>
        {#if info.dirty > 0}<span class="dirty">✚{info.dirty}</span>{/if}
        <svg class="caret" viewBox="0 0 16 16" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="m4 6 4 4 4-4" />
        </svg>
      </button>
      <button class="iconbtn" title="Switch repository" onclick={() => goto('/')}>
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
          <path d="M1.5 4.5a1 1 0 0 1 1-1h3l1.5 1.8h6.5a1 1 0 0 1 1 1v1.2M3.5 13.5l1.6-5h9.9l-1.6 5z" />
        </svg>
      </button>
    </div>

    <div class="body">
      <div class="brand">
        <span class="logo mono">PINCER</span>
        <span class="sub">指令面板</span>
      </div>

      {#if info.op}
        <!-- 搁置中的操作: 恢复入口(冲突现场在 git 仓库里, 点击回冲突页继续) -->
        <button class="resume" onclick={resume}>
          <span class="bn-dot">●</span>
          <span class="bn-text">
            <span class="bn-line">
              <span class="bn-name mono">git {info.op}</span>
              <span class="bn-zh">进行中 · 已搁置</span>
            </span>
            <span class="bn-sub">
              {session.files.length
                ? `${session.files.length} 个冲突待解决 · 点击恢复`
                : '冲突已全部解决, 点击继续 (continue)'}
            </span>
          </span>
          <svg class="bn-go" viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="m6 4 4 4-4 4" />
          </svg>
        </button>
      {/if}

      {#each actions as a, i (a.kind)}
        <button
          class="row"
          class:running={running === a.kind}
          disabled={running !== null || !!info.op}
          onclick={() => act(a.kind)}
        >
          <span class="ricon">{@html a.icon}</span>
          <span class="rtext">
            <span class="rline">
              <span class="rname mono">{a.label}</span>
              <span class="rzh">{a.zh}</span>
            </span>
            <span class="rdesc">{a.desc}</span>
          </span>
          {#if running === a.kind}
            <span class="spin"></span>
          {:else}
            <span class="kbd">⌘{i + 1}</span>
          {/if}
        </button>
      {/each}

      {#if term.entries.length}
        <div class="term">
          <div class="tabs">
            <span class="tab active">执行输出</span>
            <button class="iconbtn" title="Clear output" onclick={() => (term.entries = [])}>
              <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
                <path d="M2.5 4h11M5.5 4V2.8a.8.8 0 0 1 .8-.8h3.4a.8.8 0 0 1 .8.8V4M4 4l.7 9a1 1 0 0 0 1 .9h4.6a1 1 0 0 0 1-.9L12 4" />
              </svg>
            </button>
          </div>
          <div class="tlog mono" bind:this={termEl}>
            {#each term.entries as en, i (i)}
              {#if en.kind === 'cmd'}
                <span class="ln"><span class="pr">➜</span> <span class="cmd">{en.text}</span></span>
              {:else}
                <span class="ln {en.kind}">{en.text}</span>
              {/if}
            {/each}
            {#if !running}
              <span class="ln"><span class="pr">➜</span> <span class="cursor">█</span></span>
            {/if}
          </div>
        </div>
      {/if}
    </div>

    <div class="statusbar">
      <span class="mono sb-branch">
        <svg viewBox="0 0 16 16" width="11" height="11" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
          <circle cx="4.5" cy="3.5" r="1.6" /><circle cx="4.5" cy="12.5" r="1.6" /><circle cx="11.5" cy="6" r="1.6" />
          <path d="M4.5 5.1v5.8M11.5 7.6c0 2.4-4 1.6-6 2.6" />
        </svg>
        {info.yoursLabel}
      </span>
      <span class="spacer"></span>
      {#if running}
        <span class="sb-busy">● 执行中</span>
      {:else if info.op}
        <span class="sb-busy">● {info.op} 进行中</span>
      {:else}
        <span class="sb-ready">● 就绪</span>
      {/if}
    </div>
  </div>
{/if}

{#if dialog}
  <PickerDialog
    title={dialog.title}
    items={dialog.items}
    multi={dialog.multi}
    confirmLabel={dialog.confirm}
    onconfirm={confirmDialog}
    onclose={() => (dialog = null)}
  />
{/if}

{#if switchItems}
  <PickerDialog
    title="切换分支"
    items={switchItems}
    confirmLabel="Switch"
    onconfirm={doSwitch}
    onclose={() => (switchItems = null)}
  />
{/if}

<style>
  .win {
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--d-canvas);
    color: var(--d-text);
  }

  /* ── 顶栏 ─────────────────────────────── */
  .topbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--d-panel);
    border-bottom: 1px solid var(--d-border);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    background: var(--d-canvas);
    border: 1px solid var(--d-border);
    border-radius: 6px;
    padding: 3px 8px;
    font-size: 11px;
    color: var(--d-text);
    max-width: 46%;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .chip svg {
    color: var(--d-dim);
    flex: none;
  }

  .chip .bicon {
    color: var(--d-blue);
  }

  /* 分支 chip 是按钮(点击切换分支): 抵消全局 button 的固定高度 */
  button.chip {
    height: auto;
    text-align: left;
  }

  .chip .caret {
    margin-left: 1px;
  }

  .dirty {
    color: var(--d-amber);
    font-size: 10px;
  }

  .iconbtn {
    margin-left: auto;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: var(--d-dim);
  }

  .iconbtn:hover {
    background: var(--d-hover);
    color: var(--d-text);
  }

  /* ── 指令面板 ──────────────────────────── */
  .body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 12px 10px;
    min-height: 0;
  }

  .brand {
    display: flex;
    align-items: baseline;
    gap: 8px;
    margin: 0 2px 10px;
  }

  .logo {
    color: var(--d-orange);
    font-weight: 500;
    font-size: 14px;
    letter-spacing: 2px;
  }

  .sub {
    color: var(--d-dim);
    font-size: 11px;
  }

  /* 搁置操作的恢复横幅: 琥珀"进行中"语义, 置于指令列表之上 */
  .resume {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: auto;
    padding: 8px 10px;
    margin-bottom: 8px;
    border: 1px solid rgba(217, 163, 67, 0.35);
    border-radius: 8px;
    background: rgba(217, 163, 67, 0.08);
    color: var(--d-text);
    text-align: left;
  }

  .resume:hover {
    background: rgba(217, 163, 67, 0.15);
  }

  .bn-dot {
    color: var(--d-amber);
    flex: none;
  }

  .bn-text {
    display: flex;
    flex-direction: column;
    gap: 1px;
    flex: 1;
    min-width: 0;
  }

  .bn-line {
    display: flex;
    align-items: baseline;
    gap: 7px;
  }

  .bn-name {
    font-size: 13px;
  }

  .bn-zh {
    color: var(--d-amber);
    font-size: 11px;
  }

  .bn-sub {
    font-size: 11px;
    color: var(--d-dim);
  }

  .bn-go {
    color: var(--d-dim);
    flex: none;
  }

  .resume:hover .bn-go {
    color: var(--d-text);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: auto;
    padding: 7px 10px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: var(--d-text);
    text-align: left;
  }

  .row:hover:not(:disabled),
  .row.running {
    background: var(--d-sel);
  }

  .row:disabled:not(.running) {
    opacity: 0.45;
  }

  .ricon {
    display: grid;
    place-items: center;
    color: var(--d-dim);
    flex: none;
  }

  .row:hover:not(:disabled) .ricon,
  .row.running .ricon {
    color: var(--d-blue-lt);
  }

  .rtext {
    display: flex;
    flex-direction: column;
    gap: 1px;
    flex: 1;
    min-width: 0;
  }

  .rline {
    display: flex;
    align-items: baseline;
    gap: 7px;
  }

  .rname {
    font-size: 13px;
  }

  .rzh {
    color: var(--d-dim);
    font-size: 11px;
  }

  .row:hover:not(:disabled) .rzh,
  .row.running .rzh {
    color: var(--d-sel-text);
  }

  .rdesc {
    display: none;
    font-size: 11px;
    color: #9db4d8;
  }

  .row:hover:not(:disabled) .rdesc,
  .row.running .rdesc {
    display: block;
  }

  .kbd {
    background: var(--d-panel);
    border: 1px solid var(--d-border-strong);
    border-radius: 4px;
    padding: 1px 6px;
    font-size: 11px;
    color: var(--d-dim);
    flex: none;
  }

  .row:hover:not(:disabled) .kbd {
    background: #243252;
    border-color: #3a4c74;
    color: var(--d-sel-text);
  }

  .spin {
    width: 13px;
    height: 13px;
    border: 2px solid rgba(138, 180, 255, 0.3);
    border-top-color: var(--d-blue-lt);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    flex: none;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* ── 执行输出终端 ──────────────────────── */
  .term {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-top: 1px solid var(--d-border);
    background: var(--d-term);
    margin-left: -12px;
    margin-right: -12px;
    margin-bottom: -10px;
  }

  .tabs {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 7px 12px 0;
    border-bottom: 1px solid var(--d-panel);
    font-size: 11px;
  }

  .tab {
    color: var(--d-dim);
    padding-bottom: 6px;
  }

  .tab.active {
    color: var(--d-text);
    border-bottom: 2px solid var(--d-orange);
  }

  .tabs .iconbtn {
    margin-bottom: 4px;
  }

  .tlog {
    overflow: auto;
    max-height: 200px;
    padding: 8px 12px 10px;
    font-size: 12px;
    line-height: 1.85;
    -webkit-user-select: text;
    user-select: text;
  }

  .ln {
    display: block;
    white-space: pre-wrap;
    word-break: break-all;
  }

  .pr {
    color: var(--d-green);
  }

  .cmd {
    color: var(--d-text);
  }

  .ln.out {
    color: var(--d-dimmer);
  }

  .ln.err {
    color: var(--d-red);
  }

  .ln.ok {
    color: var(--d-green);
  }

  .ln.fail {
    color: var(--d-red);
  }

  .cursor {
    color: var(--d-orange);
    animation: blink 1s steps(1) infinite;
  }

  @keyframes blink {
    50% {
      opacity: 0;
    }
  }

  /* ── 状态栏 ───────────────────────────── */
  .statusbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 5px 12px;
    background: var(--d-panel);
    border-top: 1px solid var(--d-border);
    font-size: 11px;
    color: var(--d-dim);
  }

  .sb-branch {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .spacer {
    flex: 1;
  }

  .sb-ready {
    color: var(--d-green);
  }

  /* 琥珀承担"进行中"语义; PINCER 橙保持只在 logo / 活动 tab / 终端光标三处出现 */
  .sb-busy {
    color: var(--d-amber);
  }
</style>
