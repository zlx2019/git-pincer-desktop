<script lang="ts">
  // 菜单指令面板(IDEA New UI 暗色小窗, 见 docs/IDEA_STYLE.md 与根目录 mockup):
  // 五操作对齐 CLI, ⌘1–⌘5 快捷键, 底部终端风执行输出; 出现冲突立刻接管切大窗
  import { onMount } from 'svelte';
  import { goto, preloadCode } from '$app/navigation';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { api, type LaunchKind } from '$lib/api';
  import { rafBatcher } from '$lib/batch';
  import { t } from '$lib/i18n.svelte';
  import { pushTerm, session, term, type TermEntry } from '$lib/state.svelte';
  import { toast } from '$lib/toast.svelte';
  import { settingsUi } from '$lib/settings.svelte';
  import { compactWindow } from '$lib/win';
  import PickerDialog from '$lib/components/PickerDialog.svelte';

  interface DialogSpec {
    kind: LaunchKind;
    title: string;
    multi: boolean;
    confirm: string;
    items: { id: string; label: string; sublabel?: string; disabled?: boolean }[];
    order: string[];
    /** 乐观弹窗: 对话框先弹出, 列表数据仍在拉取中 */
    loading?: boolean;
  }

  let running: LaunchKind | null = $state(null);
  let dialog: DialogSpec | null = $state(null);
  let switchDialog: {
    loading: boolean;
    items: { id: string; label: string; sublabel?: string; disabled?: boolean }[];
  } | null = $state(null);
  let termEl: HTMLElement | undefined = $state();
  // 窗口形态就位门: set_window_form 返回后才挂载内容, 掩蔽"先换页面、后跳窗口几何"
  // 的两段突变; 只在形态真的变了时播放淡入(同形态导航保持硬切, 不平白慢一拍)
  let formReady = $state(false);
  let formChanged = $state(false);

  // 图标为静态字面量, {@html} 安全; 短名/描述文案走 i18n 词典(act-*/actd-*)
  const actions: { kind: LaunchKind; label: string; icon: string }[] = [
    {
      kind: 'pull',
      label: 'pull',
      icon: '<svg viewBox="0 0 16 16" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M8 2.5v7M4.8 6.8 8 10l3.2-3.2M3 13.5h10"/></svg>',
    },
    {
      kind: 'merge',
      label: 'merge',
      icon: '<svg viewBox="0 0 16 16" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="4.5" cy="3.5" r="1.7"/><circle cx="4.5" cy="12.5" r="1.7"/><circle cx="11.5" cy="8" r="1.7"/><path d="M4.5 5.2v5.6M4.5 6.5c0 2 2.7 1.5 5.3 1.5"/></svg>',
    },
    {
      kind: 'rebase',
      label: 'rebase',
      icon: '<svg viewBox="0 0 16 16" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M3 4h6.5M3 8h4.5M3 12h6.5M12.5 3.5v7.5M10.6 9.2l1.9 1.9 1.9-1.9"/></svg>',
    },
    {
      kind: 'cherry-pick',
      label: 'cherry-pick',
      icon: '<svg viewBox="0 0 16 16" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><circle cx="5.4" cy="11.2" r="2.1"/><circle cx="10.9" cy="11.9" r="2.1"/><path d="M5.4 9.1C5.4 6 7.6 4.6 10.2 2.6M10.9 9.8C10.6 6.9 9.7 5.4 10.2 2.6"/></svg>',
    },
    {
      kind: 'revert',
      label: 'revert',
      icon: '<svg viewBox="0 0 16 16" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M3.5 3.5v4h4"/><path d="M3.8 7.2a4.9 4.9 0 1 0 1.1-3"/></svg>',
    },
  ];

  onMount(() => {
    if (!session.info) {
      goto('/');
      return;
    }
    // 操作进行中且未被用户搁置 → 接管进冲突页; 搁置时留在菜单(顶部有恢复横幅)。
    // 切窗放在守卫之后: 重定向路径不再先切小窗又被目标页切回大窗
    if (session.info.op && !session.parked) {
      goto('/conflicts');
      return;
    }
    compactWindow()
      .then((changed) => (formChanged = changed))
      .catch(() => {})
      .finally(() => (formReady = true));
    // 冲突页代码预热: 出现冲突接管时不再现场拉取/解析模块
    preloadCode('/conflicts').catch(() => {});
    // 终端里发起的操作也要能接管: 聚焦时重探
    let unlisten: (() => void) | undefined;
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused) reprobe();
      })
      .then((u) => (unlisten = u));
    return () => unlisten?.();
  });

  // 聚焦重探限频: 进行中的不叠加, 800ms 冷却(点标题栏/快速切窗的连环焦点事件不再各起一串 git)
  let probing = false;
  let lastProbe = 0;

  /** 聚焦重探: 有操作进行中且未搁置则接管进冲突页; 搁置期间只刷新横幅数据 */
  async function reprobe() {
    if (running || !session.info) return;
    if (probing || Date.now() - lastProbe < 800) return;
    probing = true;
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
    } finally {
      probing = false;
      lastProbe = Date.now();
    }
  }

  /** 恢复被搁置的冲突解决 */
  function resume() {
    session.parked = false;
    goto('/conflicts');
  }

  /** 打开分支切换对话框(当前分支置顶且置灰): 乐观先弹 loading, 分支列表回来再填充 */
  async function askSwitch() {
    if (running || dialog || switchDialog) return;
    switchDialog = { loading: true, items: [] };
    try {
      const bs = (await api.branches()).sort((a, b) => Number(b.current) - Number(a.current));
      // 等待期间用户已按 Esc 关闭: 丢弃迟到的数据
      if (!switchDialog) return;
      switchDialog = {
        loading: false,
        items: bs.map((b) => ({
          id: b.name,
          label: b.name,
          sublabel: b.current ? 'current' : undefined,
          disabled: b.current,
        })),
      };
    } catch (e) {
      switchDialog = null;
      toast(String(e));
    }
  }

  /** 执行切换: 回执进终端, 失败原样透出 git 的说明(如工作区有未提交改动) */
  async function doSwitch(ids: string[]) {
    const name = ids[0];
    switchDialog = null;
    pushTerm({ kind: 'cmd', text: `git switch ${name}` });
    try {
      await api.switchBranch(name);
      session.info = await api.repoOpen();
      pushTerm({ kind: 'ok', text: `✔ switched to ${name}` });
    } catch (e) {
      pushTerm({ kind: 'fail', text: `✘ ${String(e)}` });
    }
  }

  /** ⌘/Ctrl + 1..5 触发对应操作(内容未挂载的就位空窗期不响应, 避免无反馈触发) */
  function hotkey(e: KeyboardEvent) {
    if (!formReady) return;
    if (!(e.metaKey || e.ctrlKey) || dialog || switchDialog || settingsUi.open || running) return;
    const idx = ['1', '2', '3', '4', '5'].indexOf(e.key);
    if (idx >= 0) {
      e.preventDefault();
      act(actions[idx].kind);
    }
  }

  /** 点操作: pull 直接执行, merge/rebase 弹分支选择, cherry-pick/revert 弹提交选择。
      对话框乐观先弹(loading 态), 数据回来再填充——点击瞬间即有视觉反馈 */
  async function act(kind: LaunchKind) {
    // 已有操作进行中(搁置状态)时不允许再发起, git 也会拒绝
    if (running || session.info?.op) return;
    const current = session.info?.yoursLabel ?? '';
    try {
      if (kind === 'pull') {
        await run(kind, []);
      } else if (kind === 'merge' || kind === 'rebase') {
        dialog = {
          kind,
          multi: false,
          confirm: kind === 'merge' ? 'Merge' : 'Rebase',
          title: kind === 'merge' ? t('dlg-merge', current) : t('dlg-rebase', current),
          items: [],
          order: [],
          loading: true,
        };
        // 当前分支置顶(IDEA 习惯), 其余保持 git 的字母序
        const bs = (await api.branches()).sort((a, b) => Number(b.current) - Number(a.current));
        // 等待期间对话框已被关闭: 丢弃迟到的数据
        if (dialog?.kind !== kind) return;
        dialog = {
          ...dialog,
          loading: false,
          items: bs.map((b) => ({
            id: b.name,
            label: b.name,
            sublabel: b.current ? 'current' : undefined,
            disabled: b.current,
          })),
          order: bs.map((b) => b.name),
        };
      } else {
        dialog = {
          kind,
          multi: true,
          confirm: kind === 'cherry-pick' ? 'Cherry-pick' : 'Revert',
          title: kind === 'cherry-pick' ? t('dlg-cherry-pick') : t('dlg-revert'),
          items: [],
          order: [],
          loading: true,
        };
        const cs = await api.commits(kind === 'cherry-pick', 30);
        if (dialog?.kind !== kind) return;
        dialog = {
          ...dialog,
          loading: false,
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
      // 取数失败: 收回乐观弹出的对话框, 错误走 toast
      if (dialog?.kind === kind) dialog = null;
      toast(String(e));
    }
  }

  /** 对话框确认: 列表为新→旧, cherry-pick 需按旧→新逐个应用, revert 保持新→旧 */
  function confirmDialog(ids: string[]) {
    if (!dialog || dialog.loading) return;
    const order = dialog.order;
    const byOrder = [...ids].sort((a, b) => order.indexOf(a) - order.indexOf(b));
    const targets = dialog.kind === 'cherry-pick' ? byOrder.reverse() : byOrder;
    const kind = dialog.kind;
    dialog = null;
    run(kind, targets);
  }

  /** 执行操作: 命令回显 + 输出流入终端(rAF 合帧); 出冲突 → 大窗冲突页, 结束打尾行 */
  async function run(kind: LaunchKind, targets: string[]) {
    running = kind;
    pushTerm({ kind: 'cmd', text: ['git', kind, ...targets].join(' ') });
    const started = Date.now();
    // 大输出(如冗长 pull)逐行入 $state 会逐行重排; 合帧后每帧一次批量落地
    const batch = rafBatcher<TermEntry>((b) => pushTerm(...b));
    const unlisten = await api.onOutput((lines) => {
      for (const l of lines) {
        batch.push({ kind: l.stream === 'stderr' ? 'err' : 'out', text: l.line });
      }
    });
    try {
      const outcome = await api.launchOp(kind, targets);
      batch.drain();
      session.info = await api.repoOpen();
      if (outcome.kind === 'conflicts') {
        session.files = outcome.files;
        await goto('/conflicts');
      } else if (outcome.kind === 'cleanDone') {
        const secs = ((Date.now() - started) / 1000).toFixed(1);
        pushTerm({ kind: 'ok', text: t('term-done', secs) });
      } else {
        pushTerm({ kind: 'fail', text: `✘ ${outcome.message}` });
      }
    } catch (e) {
      batch.drain();
      pushTerm({ kind: 'fail', text: `✘ ${String(e)}` });
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

{#if session.info && formReady}
  {@const info = session.info}
  <div class="win" class:page-in={formChanged}>
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
      <!-- 换仓库 = 文件夹 + 内嵌左箭头(回打开页); 设置入口不放顶栏(2026-07-12 Zero 定),
           走 ⌘(Ctrl)+, 与 macOS 应用菜单 -->
      <button class="iconbtn" title="Switch repository" onclick={() => goto('/')}>
        <svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
          <path d="M14.5 12.2a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1V3.8a1 1 0 0 1 1-1h3l1.4 1.7h6.6a1 1 0 0 1 1 1z" />
          <path d="M10.4 9.4H5.8M7.6 7.6 5.8 9.4l1.8 1.8" />
        </svg>
      </button>
    </div>

    <div class="body">
      <div class="brand">
        <span class="logo mono">PINCER</span>
        <span class="sub">{t('brand-sub')}</span>
      </div>

      {#if info.op}
        <!-- 搁置中的操作: 恢复入口(冲突现场在 git 仓库里, 点击回冲突页继续) -->
        <button class="resume" onclick={resume}>
          <span class="bn-dot">●</span>
          <span class="bn-text">
            <span class="bn-line">
              <span class="bn-name mono">git {info.op}</span>
              <span class="bn-zh">{t('resume-state')}</span>
            </span>
            <span class="bn-sub">
              {session.files.length
                ? t('resume-pending', session.files.length)
                : t('resume-done')}
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
              <span class="rzh">{t(`act-${a.kind}`)}</span>
            </span>
            <span class="rdesc">{t(`actd-${a.kind}`)}</span>
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
            <span class="tab active">{t('term-tab')}</span>
            <button class="iconbtn" title="Clear output" onclick={() => (term.entries = [])}>
              <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round">
                <path d="M2.5 4h11M5.5 4V2.8a.8.8 0 0 1 .8-.8h3.4a.8.8 0 0 1 .8.8V4M4 4l.7 9a1 1 0 0 0 1 .9h4.6a1 1 0 0 0 1-.9L12 4" />
              </svg>
            </button>
          </div>
          <div class="tlog mono" bind:this={termEl}>
            {#each term.entries as en (en.id)}
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
        <span class="sb-busy">{t('sb-running')}</span>
      {:else if info.op}
        <span class="sb-busy">{t('sb-oping', info.op)}</span>
      {:else}
        <span class="sb-ready">{t('sb-ready')}</span>
      {/if}
    </div>
  </div>
{/if}

{#if dialog}
  <PickerDialog
    title={dialog.title}
    items={dialog.items}
    multi={dialog.multi}
    loading={dialog.loading ?? false}
    confirmLabel={dialog.confirm}
    onconfirm={confirmDialog}
    onclose={() => (dialog = null)}
  />
{/if}

{#if switchDialog}
  <PickerDialog
    title={t('dlg-switch')}
    items={switchDialog.items}
    loading={switchDialog.loading}
    confirmLabel="Switch"
    onconfirm={doSwitch}
    onclose={() => (switchDialog = null)}
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

  .iconbtn,
  .chip {
    transition:
      background-color 0.12s ease-out,
      color 0.12s ease-out;
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
    flex: none;
    padding: 8px 10px;
    margin-bottom: 8px;
    border: 1px solid rgba(217, 163, 67, 0.35);
    border-radius: 8px;
    background: rgba(217, 163, 67, 0.08);
    color: var(--d-text);
    text-align: left;
    transition: background-color 0.12s ease-out;
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
    /* 行高恒定(描述行常驻占位), 空间不足时收缩的是可滚动的终端块 */
    flex: none;
    transition: background-color 0.12s ease-out;
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
    transition: color 0.12s ease-out;
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
    transition: color 0.12s ease-out;
  }

  .row:hover:not(:disabled) .rzh,
  .row.running .rzh {
    color: var(--d-sel-text);
  }

  /* 描述行常驻占位(行高恒为两行), hover 仅做透明度渐显——
     原先 display:none→block 的切换会把下方行与终端块整体推挤, 鼠标扫过列表时逐行抖动 */
  .rdesc {
    font-size: 11px;
    color: var(--d-desc);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    opacity: 0;
    transition: opacity 0.15s ease-out;
  }

  .row:hover:not(:disabled) .rdesc,
  .row.running .rdesc {
    opacity: 1;
  }

  .kbd {
    background: var(--d-panel);
    border: 1px solid var(--d-border-strong);
    border-radius: 4px;
    padding: 1px 6px;
    font-size: 11px;
    color: var(--d-dim);
    flex: none;
    transition:
      background-color 0.12s ease-out,
      border-color 0.12s ease-out,
      color 0.12s ease-out;
  }

  .row:hover:not(:disabled) .kbd {
    background: var(--d-sel-dim);
    border-color: var(--d-sel-dim-border);
    color: var(--d-sel-text);
  }

  .spin {
    width: 13px;
    height: 13px;
    border: 2px solid var(--d-spin-track);
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
