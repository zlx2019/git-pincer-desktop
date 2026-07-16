<script lang="ts">
  // 打开仓库页(IDEA 暗色小窗): 目录选择 / 拖拽 / 最近列表
  import { onMount } from 'svelte';
  import { goto, preloadCode } from '$app/navigation';
  import { open } from '@tauri-apps/plugin-dialog';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { api, type RecentRepo } from '$lib/api';
  import { t } from '$lib/i18n.svelte';
  import { session } from '$lib/state.svelte';
  import { toast } from '$lib/toast.svelte';
  import { compactWindow } from '$lib/win';

  let recent: RecentRepo[] = $state([]);
  // 空态提示等列表拿到后再出, 避免 IPC 返回前闪一下
  let recentLoaded = $state(false);
  let opening = $state(false);
  // 窗口形态就位门: set_window_form 返回后才挂载内容(大窗回打开页先归位小窗);
  // 只在形态真的变了时播放淡入(菜单↔打开页同为小窗, 往返保持硬切)
  let formReady = $state(false);
  let formChanged = $state(false);

  /** 拉取最近列表(挂载与窗口聚焦时): 外部删除/移动仓库即时反映置灰 */
  function refreshRecent() {
    api
      .recentRepos()
      .then((r) => (recent = r))
      .catch(() => {})
      .finally(() => (recentLoaded = true));
  }

  onMount(() => {
    compactWindow()
      .then((changed) => (formChanged = changed))
      .catch(() => {})
      .finally(() => (formReady = true));
    // 两个可能的下一站预热: 打开仓库时目标页已就绪
    preloadCode('/menu').catch(() => {});
    preloadCode('/conflicts').catch(() => {});
    refreshRecent();
    // 拖拽文件夹进窗口 = 打开仓库
    let unlisten: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((e) => {
        if (e.payload.type === 'drop' && e.payload.paths.length > 0) {
          openRepo(e.payload.paths[0]);
        }
      })
      .then((u) => (unlisten = u));
    return () => unlisten?.();
  });

  /** 弹目录选择器后打开 */
  async function pickAndOpen() {
    const dir = await open({ directory: true, title: 'Open Git Repository' });
    if (typeof dir === 'string') await openRepo(dir);
  }

  /** 打开仓库: 有进行中的操作(冲突)直接接管, 否则进菜单 */
  async function openRepo(path: string) {
    if (opening) return;
    opening = true;
    try {
      session.info = await api.repoOpen(path);
      session.files = session.info.op ? await api.conflicts() : [];
      await goto(session.info.op ? '/conflicts' : '/menu');
    } catch (e) {
      toast(String(e));
    } finally {
      opening = false;
    }
  }

  /** 从最近列表移除(只删记录, 不动仓库本身) */
  async function removeRecent(path: string) {
    try {
      recent = await api.recentRemove(path);
    } catch (e) {
      toast(String(e));
    }
  }

  /** 路径末段作为展示名 */
  function basename(path: string): string {
    return path.split('/').filter(Boolean).pop() ?? path;
  }
</script>

<svelte:window onfocus={refreshRecent} />

{#if formReady}
  <main class="win" class:page-in={formChanged}>
    <div class="hero">
      <!-- v2 logo 图形(几何同 assets/icon.svg): 挖空处用页面底色 token, 亮暗主题自适应 -->
      <svg class="mark" viewBox="146 146 732 732" width="72" height="72" aria-hidden="true">
        <g stroke="var(--d-orange)" stroke-width="56" stroke-linecap="round">
          <line x1="512" y1="236" x2="512" y2="194" />
          <line x1="617.6" y1="257" x2="633.7" y2="218.2" />
          <line x1="707.2" y1="316.8" x2="736.9" y2="287.1" />
          <line x1="767" y1="406.4" x2="805.8" y2="390.3" />
          <line x1="788" y1="512" x2="830" y2="512" />
          <line x1="767" y1="617.6" x2="805.8" y2="633.7" />
          <line x1="707.2" y1="707.2" x2="736.9" y2="736.9" />
          <line x1="617.6" y1="767" x2="633.7" y2="805.8" />
          <line x1="512" y1="788" x2="512" y2="830" />
          <line x1="406.4" y1="767" x2="390.3" y2="805.8" />
          <line x1="316.8" y1="707.2" x2="287.1" y2="736.9" />
          <line x1="257" y1="617.6" x2="218.2" y2="633.7" />
          <line x1="236" y1="512" x2="194" y2="512" />
          <line x1="257" y1="406.4" x2="218.2" y2="390.3" />
          <line x1="316.8" y1="316.8" x2="287.1" y2="287.1" />
          <line x1="406.4" y1="257" x2="390.3" y2="218.2" />
        </g>
        <circle cx="512" cy="512" r="300" fill="var(--d-orange)" />
        <g stroke="var(--d-canvas)" stroke-width="30" stroke-linecap="round">
          <line x1="414" y1="660" x2="512" y2="548" />
          <line x1="610" y1="660" x2="512" y2="548" />
          <line x1="512" y1="548" x2="512" y2="352" />
        </g>
        <circle cx="414" cy="660" r="32" fill="var(--d-canvas)" />
        <circle cx="610" cy="660" r="32" fill="var(--d-canvas)" />
        <circle cx="512" cy="352" r="40" fill="var(--d-canvas)" />
        <circle cx="512" cy="548" r="52" fill="var(--d-canvas)" />
        <circle cx="512" cy="548" r="22" fill="var(--d-orange)" />
      </svg>
      <h1 class="logo mono">PINCER</h1>
      <button class="primary" disabled={opening} onclick={pickAndOpen}>Open Repository…</button>
    </div>

    <!-- IDEA Welcome 风格: 分割线下方列表区常驻, 空列表时以提示占位不显空旷 -->
    <section class="recent">
      <h2>RECENT</h2>
      {#if recent.length}
        {#each recent as r (r.path)}
          <div class="rrow-wrap" class:missing={r.missing}>
            <button
              class="rrow"
              onclick={() => (r.missing ? toast(t('open-missing')) : openRepo(r.path))}
            >
              <span class="rname">{basename(r.path)}</span>
              <span class="rpath mono">{r.path}</span>
            </button>
            <button class="rdel" title="Remove from recent" onclick={() => removeRecent(r.path)}>
              <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"><path d="m4.5 4.5 7 7M11.5 4.5l-7 7" /></svg>
            </button>
          </div>
        {/each}
      {:else if recentLoaded}
        <p class="rempty">{t('open-recent-empty')}</p>
      {/if}
    </section>
  </main>
{/if}

<style>
  .win {
    height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    background: var(--d-canvas);
    color: var(--d-text);
    padding: 0 18px;
    overflow: auto;
  }

  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    margin-top: 76px;
  }

  .logo {
    color: var(--d-orange);
    font-size: 26px;
    font-weight: 500;
    letter-spacing: 5px;
    margin: 0 0 12px;
  }

  .primary {
    background: var(--d-blue);
    border: 1px solid var(--d-blue);
    color: #ffffff;
    transition:
      background-color 0.12s ease-out,
      border-color 0.12s ease-out;
  }

  .primary:hover:not(:disabled) {
    background: #2f63d6;
    border-color: #2f63d6;
  }

  /* IDEA Welcome 语法: 一条分割线划出列表区, 行静止透明、hover 才提亮 */
  .recent {
    width: 100%;
    max-width: 360px;
    margin-top: 34px;
    padding-top: 14px;
    border-top: 1px solid var(--d-border);
  }

  .recent h2 {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 1px;
    color: var(--d-dimmer);
    margin: 0 6px 6px;
  }

  .rrow-wrap {
    position: relative;
  }

  .rrow {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    height: auto;
    padding: 6px 28px 6px 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--d-text);
    text-align: left;
    transition: background-color 0.12s ease-out;
  }

  .rempty {
    margin: 2px 0 0;
    padding: 6px 8px;
    font-size: 11px;
    color: var(--d-dimmer);
  }

  /* 目录已删除/移动: IDEA 式整行置灰(保留展示, 待用户手动移除) */
  .rrow-wrap.missing .rname {
    color: var(--d-dim);
  }

  .rrow-wrap.missing .rpath {
    color: var(--d-dimmer);
  }

  /* hover 提亮挂在包装层: 悬停删除键时行高亮不熄灭 */
  .rrow-wrap:hover .rrow {
    background: var(--d-hover);
  }

  /* IDEA Welcome 页习惯: 删除键仅 hover/聚焦时浮现 */
  .rdel {
    position: absolute;
    top: 50%;
    right: 6px;
    transform: translateY(-50%);
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--d-dim);
    display: grid;
    place-items: center;
    opacity: 0;
    pointer-events: none;
    transition: opacity 0.12s ease-out;
  }

  .rrow-wrap:hover .rdel,
  .rdel:focus-visible {
    opacity: 1;
    pointer-events: auto;
  }

  .rdel:hover {
    background: var(--d-sel);
    color: var(--d-text);
  }

  .rname {
    font-weight: 600;
    font-size: 12px;
  }

  .rpath {
    font-size: 10px;
    color: var(--d-dim);
  }
</style>
