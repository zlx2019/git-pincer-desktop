<script lang="ts">
  // 全局布局: 主题、首帧就绪后显窗(防启动白闪)、生产交互加固、toast 层
  import '$lib/theme.css';
  import { onMount } from 'svelte';
  import { dev } from '$app/environment';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { toasts } from '$lib/toast.svelte';

  let { children } = $props();

  onMount(() => {
    // 窗口配置 visible:false: 首帧绘制完成后再显示, 启动不再闪白底/无样式内容
    requestAnimationFrame(() => {
      const win = getCurrentWindow();
      win
        .show()
        .then(() => win.setFocus())
        .catch(() => {});
    });
  });

  /** 生产加固: 桌面应用不出现浏览器右键菜单;
      例外: 可编辑区域, 以及存在文本选区时(保留原生 Copy 菜单) */
  function contextmenu(e: MouseEvent) {
    if (dev) return;
    const t = e.target as HTMLElement | null;
    if (t?.closest('input, textarea, [contenteditable="true"]')) return;
    const sel = window.getSelection();
    if (sel && !sel.isCollapsed) return;
    e.preventDefault();
  }

  /** 生产加固: 拦截 webview 刷新/打印快捷键(WebView2 默认启用浏览器加速键;
      刷新会丢会话状态回打开页) */
  function hardenKeys(e: KeyboardEvent) {
    if (dev) return;
    const mod = e.metaKey || e.ctrlKey;
    if (e.key === 'F5' || (mod && ['r', 'R', 'p', 'P'].includes(e.key))) {
      e.preventDefault();
    }
  }
</script>

<svelte:window oncontextmenu={contextmenu} onkeydown={hardenKeys} />

{@render children()}

{#if toasts.list.length}
  <div class="toasts">
    {#each toasts.list as t (t.id)}
      <div class="toast">{t.msg}</div>
    {/each}
  </div>
{/if}

<style>
  /* IDEA 通知气泡习惯: 右下角、状态栏/底栏之上, 主题 token 配色 */
  .toasts {
    position: fixed;
    right: 12px;
    bottom: 50px;
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .toast {
    background: var(--d-panel);
    border: 1px solid var(--d-border-strong);
    color: var(--d-text);
    padding: 8px 14px;
    border-radius: 8px;
    box-shadow: 0 6px 24px rgba(0, 0, 0, 0.45);
    max-width: 380px;
    font-size: 12px;
    -webkit-user-select: text;
    user-select: text;
    animation: toast-in 0.15s ease-out;
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
