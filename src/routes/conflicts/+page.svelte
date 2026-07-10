<script lang="ts">
  // Conflicts 文件列表页(参考图 1): 多选、目录分组、Accept、二进制对话框、Continue/Abort
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { SvelteSet } from 'svelte/reactivity';
  import { confirm } from '@tauri-apps/plugin-dialog';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { api, opTitleSegments, type FileRow, type OutputLine } from '$lib/api';
  import { largeWindow } from '$lib/win';
  import { session } from '$lib/state.svelte';
  import { toast } from '$lib/toast.svelte';
  import BinaryDialog from '$lib/components/BinaryDialog.svelte';

  const selected = new SvelteSet<string>();
  let lastIndex = -1;
  let groupByDir = $state(false);
  let binaryPath: string | null = $state(null);
  let round = $state(1);
  let running = $state(false);
  let doneBanner = $state(false);
  let outputLines = $state<OutputLine[]>([]);
  let consoleEl: HTMLElement | undefined = $state();

  onMount(() => {
    // 直接刷新进入(dev 场景)时回打开页
    if (!session.info) {
      goto('/');
      return;
    }
    // 冲突处理使用大窗
    largeWindow().catch(() => {});
    // 冲突在终端/IDE 里产生: 窗口重获焦点时自动重探仓库状态
    let unlisten: (() => void) | undefined;
    getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused) reprobe();
      })
      .then((u) => (unlisten = u));
    return () => unlisten?.();
  });

  /** 重探仓库与冲突列表(窗口聚焦 / 手动 Refresh); 列表没变化就保住选择状态 */
  async function reprobe() {
    if (running || !session.info) return;
    try {
      const info = await api.repoOpen();
      session.info = info;
      const files = info.op ? await api.conflicts() : [];
      if (JSON.stringify(files) !== JSON.stringify(session.files)) {
        session.files = files;
        selected.clear();
        lastIndex = -1;
      }
      // 新一轮操作开始 → 清掉上次的完成横幅
      if (info.op) doneBanner = false;
    } catch (e) {
      toast(String(e));
    }
  }

  // 目录分组视图; 未分组时退化为单组
  const groups = $derived.by(() => {
    if (!groupByDir) return [{ dir: null as string | null, files: session.files }];
    const map = new Map<string, FileRow[]>();
    for (const f of session.files) {
      const i = f.path.lastIndexOf('/');
      const dir = i >= 0 ? f.path.slice(0, i) : '.';
      map.set(dir, [...(map.get(dir) ?? []), f]);
    }
    return [...map.entries()]
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([dir, files]) => ({ dir: dir as string | null, files }));
  });

  // 可见顺序的平铺列表(shift 范围选择用)
  const flat = $derived(groups.flatMap((g) => g.files));

  // 恰好单选时的那一行
  const single = $derived(
    selected.size === 1 ? session.files.find((f) => selected.has(f.path)) : undefined
  );

  // Merge... 不可用的原因(空串 = 可用)
  const mergeReason = $derived(
    selected.size !== 1
      ? 'Select exactly one file to merge'
      : single && (single.yours === 'deleted' || single.theirs === 'deleted')
        ? 'Deleted files can only be resolved by accepting a side'
        : ''
  );

  /** 单击选择: 支持 ⌘/Ctrl 增选与 Shift 范围选 */
  function clickRow(e: MouseEvent, path: string, index: number) {
    if (e.shiftKey && lastIndex >= 0) {
      if (!e.metaKey && !e.ctrlKey) selected.clear();
      const [a, b] = [Math.min(lastIndex, index), Math.max(lastIndex, index)];
      for (let i = a; i <= b; i++) selected.add(flat[i].path);
    } else if (e.metaKey || e.ctrlKey) {
      if (selected.has(path)) selected.delete(path);
      else selected.add(path);
      lastIndex = index;
    } else {
      selected.clear();
      selected.add(path);
      lastIndex = index;
    }
  }

  /** 双击 = 选中并 Merge... */
  function dblclickRow(path: string, index: number) {
    selected.clear();
    selected.add(path);
    lastIndex = index;
    mergeSelected();
  }

  /** 打开三栏(文本)或 pick-one 对话框(二进制) */
  function mergeSelected() {
    if (mergeReason) {
      toast(mergeReason);
      return;
    }
    if (!single) return;
    if (single.binary) {
      binaryPath = single.path;
      return;
    }
    goto(`/merge?path=${encodeURIComponent(single.path)}`);
  }

  /** 选中文件整侧取用 */
  async function accept(side: 'yours' | 'theirs') {
    const paths = [...selected];
    if (!paths.length) return;
    try {
      await api.acceptSide(paths, side);
      await refresh();
    } catch (e) {
      toast(String(e));
    }
  }

  /** 重新拉取冲突列表 */
  async function refresh() {
    session.files = await api.conflicts();
    selected.clear();
    lastIndex = -1;
  }

  /** 驱动 --continue: 输出走事件流, 结果分 完成/下一轮/失败 */
  async function doContinue() {
    running = true;
    outputLines = [];
    const unlisten = await api.onOutput((l) => outputLines.push(l));
    try {
      const outcome = await api.continueOp();
      if (outcome.kind === 'done') {
        session.info = await api.repoOpen();
        doneBanner = true;
      } else if (outcome.kind === 'nextRound') {
        round += 1;
        session.files = outcome.files;
        toast(`Round ${round}: ${outcome.files.length} conflicting file(s)`);
      } else {
        toast(outcome.message);
      }
    } catch (e) {
      toast(String(e));
    } finally {
      unlisten();
      running = false;
    }
  }

  /** 中止操作(确认后) */
  async function doAbort() {
    const info = session.info;
    if (!info?.op) return;
    const ok = await confirm(
      `Abort the ${info.op}? All conflict resolutions in this operation will be lost.`,
      { title: 'Abort', kind: 'warning' }
    );
    if (!ok) return;
    try {
      await api.abortOp();
      session.info = await api.repoOpen();
      session.files = [];
    } catch (e) {
      toast(String(e));
    }
  }

  // 控制台自动滚到底
  $effect(() => {
    void outputLines.length;
    if (consoleEl) consoleEl.scrollTop = consoleEl.scrollHeight;
  });

  /** 状态首字母大写 */
  function cap(s: string): string {
    return s.charAt(0).toUpperCase() + s.slice(1);
  }

  /** 拆路径为 (文件名, 目录) */
  function splitPath(path: string): [string, string] {
    const i = path.lastIndexOf('/');
    return i >= 0 ? [path.slice(i + 1), path.slice(0, i)] : [path, ''];
  }
</script>

{#if session.info}
  {@const info = session.info}
  <div class="dialog">
    <header>
      <h1>Conflicts</h1>
      <p class="subtitle">
        {#if round > 1}<span class="dim">Round {round} — </span>{/if}
        {#each opTitleSegments(info) as seg, i (i)}
          {#if seg.bold}<b>{seg.text}</b>{:else}{seg.text}{/if}
        {/each}
      </p>
    </header>

    {#if !info.op}
      <div class="panel">
        <h2>{doneBanner ? 'Operation completed successfully' : 'No merge operation in progress'}</h2>
        <p class="dim mono">{info.root}</p>
        {#if !doneBanner}
          <p class="dim guide">
            Start a merge, rebase, cherry-pick or revert in this repository from your terminal or
            IDE. Conflicts appear here automatically when you come back to this window.
          </p>
          <p class="dim mono example">e.g. git merge &lt;branch&gt;</p>
        {/if}
        {#if outputLines.length}
          <pre class="console mono" bind:this={consoleEl}>{#each outputLines as l, i (i)}<span
                class={l.stream}>{l.line}
</span>{/each}</pre>
        {/if}
        <div class="row-buttons">
          <button onclick={() => goto('/menu')}>Close</button>
          <button class="primary" onclick={reprobe}>Refresh</button>
        </div>
      </div>
    {:else if session.files.length === 0}
      <div class="panel">
        <h2>All conflicts resolved</h2>
        <p class="dim">Every file is staged. Continue the {info.op} to finish this round.</p>
        {#if outputLines.length}
          <pre class="console mono" bind:this={consoleEl}>{#each outputLines as l, i (i)}<span
                class={l.stream}>{l.line}
</span>{/each}</pre>
        {/if}
        <div class="row-buttons">
          <button disabled={running} onclick={doAbort}>Abort</button>
          <button class="primary" disabled={running} onclick={doContinue}>
            {running ? 'Running…' : `Continue (git ${info.op} --continue)`}
          </button>
        </div>
      </div>
    {:else}
      <div class="body">
        <div class="table-wrap">
          <table>
            <thead>
              <tr>
                <th>Name</th>
                <th>Yours{info.yoursLabel ? ` (${info.yoursLabel})` : ''}</th>
                <th>Theirs{info.theirsLabel ? ` (${info.theirsLabel})` : ''}</th>
              </tr>
            </thead>
            <tbody>
              {#each groups as g (g.dir ?? '')}
                {#if g.dir !== null}
                  <tr class="group-row"><td colspan="3">{g.dir}/</td></tr>
                {/if}
                {#each g.files as f (f.path)}
                  {@const [fname, fdir] = splitPath(f.path)}
                  {@const index = flat.indexOf(f)}
                  <tr
                    class:selected={selected.has(f.path)}
                    onclick={(e) => clickRow(e, f.path, index)}
                    ondblclick={() => dblclickRow(f.path, index)}
                  >
                    <td class="name">
                      <svg class="ficon" viewBox="0 0 16 16" width="13" height="13">
                        <path
                          fill="none"
                          stroke="currentColor"
                          d="M3.5 1.5h6l3 3v10h-9zM9.5 1.5v3h3"
                        />
                      </svg>
                      <span class="fname">{fname}</span>
                      {#if fdir && !groupByDir}<span class="fdir dim">{fdir}</span>{/if}
                      {#if f.binary}<span class="badge dim">binary</span>{/if}
                    </td>
                    <td>{cap(f.yours)}</td>
                    <td>{cap(f.theirs)}</td>
                  </tr>
                {/each}
              {/each}
            </tbody>
          </table>
        </div>
        <div class="side-buttons">
          <button disabled={!selected.size} onclick={() => accept('yours')}>Accept Yours</button>
          <button disabled={!selected.size} onclick={() => accept('theirs')}>Accept Theirs</button>
          <button
            class="primary"
            disabled={mergeReason !== ''}
            title={mergeReason}
            onclick={mergeSelected}>Merge...</button
          >
        </div>
      </div>
      <footer>
        <label><input type="checkbox" bind:checked={groupByDir} /> Group files by directory</label>
        <span class="spacer"></span>
        <button onclick={doAbort}>Abort</button>
        <button onclick={() => goto('/menu')}>Close</button>
      </footer>
    {/if}
  </div>
{/if}

{#if binaryPath}
  <BinaryDialog
    path={binaryPath}
    onclose={() => (binaryPath = null)}
    onresolved={async () => {
      binaryPath = null;
      await refresh();
    }}
  />
{/if}

<style>
  .dialog {
    height: 100vh;
    display: flex;
    flex-direction: column;
    padding: 14px 16px;
    gap: 12px;
  }

  header h1 {
    font-size: 15px;
    font-weight: 600;
    margin: 0 0 8px;
    text-align: center;
  }

  .subtitle {
    margin: 0;
  }

  .body {
    flex: 1;
    display: flex;
    gap: 12px;
    min-height: 0;
  }

  .table-wrap {
    flex: 1;
    overflow: auto;
    border: 1px solid var(--d-border);
    border-radius: 8px;
    background: #26282b;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th {
    text-align: left;
    font-weight: 500;
    padding: 5px 10px;
    border-bottom: 1px solid var(--d-border);
    position: sticky;
    top: 0;
    background: var(--d-panel);
    color: var(--d-dim);
    font-size: 12px;
  }

  th:first-child {
    width: 55%;
  }

  td {
    padding: 4px 10px;
    white-space: nowrap;
  }

  tbody tr:hover:not(.group-row) {
    background: var(--d-hover);
  }

  tbody tr.selected,
  tbody tr.selected:hover {
    background: var(--d-sel);
  }

  .group-row td {
    background: var(--d-panel);
    color: var(--d-dim);
    font-size: 12px;
    padding: 3px 10px;
  }

  .name {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .ficon {
    color: var(--d-dim);
    flex: none;
  }

  .fname {
    color: var(--d-red);
  }

  .fdir {
    font-size: 12px;
  }

  .badge {
    font-size: 11px;
    border: 1px solid var(--d-border-strong);
    border-radius: 4px;
    padding: 0 5px;
  }

  .side-buttons {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 138px;
  }

  .side-buttons button {
    width: 100%;
  }

  footer {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .spacer {
    flex: 1;
  }

  .panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
  }

  .panel h2 {
    font-size: 16px;
    margin: 0;
  }

  .row-buttons {
    display: flex;
    gap: 10px;
    margin-top: 8px;
  }

  .guide {
    max-width: 460px;
    text-align: center;
    line-height: 1.5;
    margin: 2px 0 0;
  }

  .example {
    font-size: 12px;
    margin: 0;
  }

</style>
