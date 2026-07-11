<script lang="ts">
  // 全局布局: 引入主题并渲染 toast 层
  import '$lib/theme.css';
  import { toasts } from '$lib/toast.svelte';

  let { children } = $props();
</script>

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
