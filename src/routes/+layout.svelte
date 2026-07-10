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
  .toasts {
    position: fixed;
    top: 12px;
    right: 12px;
    z-index: 100;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .toast {
    background: #3c3f41;
    color: #ffffff;
    padding: 8px 14px;
    border-radius: 6px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.25);
    max-width: 420px;
    font-size: 12px;
    -webkit-user-select: text;
    user-select: text;
  }
</style>
