<script lang="ts">
  // 打开仓库页(IDEA 暗色小窗): 目录选择 / 拖拽 / 最近列表
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';
  import { open } from '@tauri-apps/plugin-dialog';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { api } from '$lib/api';
  import { session } from '$lib/state.svelte';
  import { toast } from '$lib/toast.svelte';
  import { compactWindow } from '$lib/win';

  let recent: string[] = $state([]);
  let opening = $state(false);

  onMount(() => {
    compactWindow().catch(() => {});
    api
      .recentRepos()
      .then((r) => (recent = r))
      .catch(() => {});
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

  /** 路径末段作为展示名 */
  function basename(path: string): string {
    return path.split('/').filter(Boolean).pop() ?? path;
  }
</script>

<main class="win">
  <div class="hero">
    <h1 class="logo mono">PINCER</h1>
    <p class="tagline">IDEA-style Git conflict resolver</p>
    <button class="primary" disabled={opening} onclick={pickAndOpen}>Open Repository…</button>
    <p class="hint">…或把仓库文件夹拖进这个窗口</p>
  </div>

  {#if recent.length}
    <section class="recent">
      <h2>RECENT</h2>
      {#each recent as path (path)}
        <button class="rrow" onclick={() => openRepo(path)}>
          <span class="rname">{basename(path)}</span>
          <span class="rpath mono">{path}</span>
        </button>
      {/each}
    </section>
  {/if}
</main>

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
    margin-top: 96px;
  }

  .logo {
    color: var(--d-orange);
    font-size: 26px;
    font-weight: 500;
    letter-spacing: 5px;
    margin: 0;
  }

  .tagline {
    color: var(--d-dim);
    font-size: 12px;
    margin: 0 0 14px;
  }

  .primary {
    background: var(--d-blue);
    border: 1px solid var(--d-blue);
    color: #ffffff;
  }

  .primary:hover:not(:disabled) {
    background: #2f63d6;
    border-color: #2f63d6;
  }

  .hint {
    color: var(--d-dimmer);
    font-size: 11px;
    margin: 2px 0 0;
  }

  .recent {
    width: 100%;
    max-width: 360px;
    margin-top: 34px;
  }

  .recent h2 {
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 1px;
    color: var(--d-dimmer);
    margin: 0 6px 6px;
  }

  .rrow {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    height: auto;
    padding: 6px 8px;
    border: none;
    border-radius: 7px;
    background: transparent;
    color: var(--d-text);
    text-align: left;
  }

  .rrow:hover {
    background: var(--d-hover);
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
