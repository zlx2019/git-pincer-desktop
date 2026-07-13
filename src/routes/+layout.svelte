<script lang="ts">
  // 全局布局: 主题、首帧就绪后显窗(防启动白闪)、生产交互加固、toast 层
  import '$lib/theme.css';
  import { onMount } from 'svelte';
  import { dev } from '$app/environment';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { api } from '$lib/api';
  import { loadSettings, settingsUi } from '$lib/settings.svelte';
  import { toasts } from '$lib/toast.svelte';

  let { children } = $props();

  onMount(() => {
    // 窗口配置 visible:false: 等设置就位(data-theme 已同步落 DOM, 浅色用户不再暗→亮闪变)
    // 后立即显示。不可等 rAF——隐藏窗口在 WKWebView 上不产帧, rAF 回调永不触发,
    // 窗口会永远不出现(v0.1.0 macOS 实机回归); 早于首帧显窗也无闪变, 透出的是
    // Rust 在 setup 里已按主题就位的原生底色。loadSettings 挂掉则 150ms 兜底显窗
    const fallback = new Promise((r) => setTimeout(r, 150));
    Promise.race([loadSettings(), fallback]).then(() => {
      const win = getCurrentWindow();
      win
        .show()
        .then(() => win.setFocus())
        .catch(() => {});
    });
    // "设置…"(macOS 应用菜单 / 各平台托盘菜单) → 弹全局设置对话框
    let unlisten: (() => void) | undefined;
    api
      .onOpenSettings(() => (settingsUi.open = true))
      .then((u) => (unlisten = u))
      .catch(() => {});
    return () => unlisten?.();
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
    // ⌘,/Ctrl+, 开设置(菜单加速键的 webview 侧双保险; Windows/Linux 无应用菜单全靠这里)
    if ((e.metaKey || e.ctrlKey) && e.key === ',') {
      e.preventDefault();
      settingsUi.open = true;
      return;
    }
    if (dev) return;
    const mod = e.metaKey || e.ctrlKey;
    if (e.key === 'F5' || (mod && ['r', 'R', 'p', 'P'].includes(e.key))) {
      e.preventDefault();
    }
  }
</script>

<svelte:window oncontextmenu={contextmenu} onkeydown={hardenKeys} />

{@render children()}

{#if settingsUi.open}
  <!-- 动态 import: 对话框携带 plugin-opener/getVersion, 静态引入会被拖进 boot chunk -->
  {#await import('$lib/components/SettingsDialog.svelte') then { default: SettingsDialog }}
    <SettingsDialog onclose={() => (settingsUi.open = false)} />
  {/await}
{/if}

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
