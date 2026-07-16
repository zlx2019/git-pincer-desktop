<script lang="ts">
  // 应用内确认对话框(IDEA 暗色): 替代系统原生 confirm, 与主题一致
  // Esc / 点遮罩 = 取消, Enter = 确认; danger 时确认键用警示红
  let {
    title,
    message,
    confirmLabel,
    danger = false,
    onconfirm,
    onclose,
  }: {
    title: string;
    message: string;
    confirmLabel: string;
    danger?: boolean;
    onconfirm: () => void;
    onclose: () => void;
  } = $props();

  function keydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onclose();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      onconfirm();
    }
  }
</script>

<svelte:window onkeydown={keydown} />

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal" role="dialog" aria-modal="true" aria-label={title}>
    <h3>{title}</h3>
    <p class="msg dim">{message}</p>
    <footer>
      <span class="spacer"></span>
      <button onclick={onclose}>Cancel</button>
      <button class="primary" class:danger onclick={onconfirm}>{confirmLabel}</button>
    </footer>
  </div>
</div>

<style>
  /* 入场动效只动 opacity/transform(合成器属性), 与其余对话框同款 */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 60;
    animation: overlay-in 0.12s ease-out;
  }

  .modal {
    width: 420px;
    max-width: calc(100vw - 40px);
    background: var(--d-panel);
    border: 1px solid var(--d-border-strong);
    border-radius: 10px;
    padding: 18px 20px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
    color: var(--d-text);
    animation: modal-in 0.15s ease-out;
  }

  @keyframes overlay-in {
    from {
      opacity: 0;
    }
  }

  @keyframes modal-in {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
  }

  h3 {
    margin: 0 0 10px;
    font-size: 15px;
  }

  .msg {
    margin: 0;
    line-height: 1.55;
  }

  footer {
    display: flex;
    gap: 8px;
    margin-top: 18px;
  }

  .spacer {
    flex: 1;
  }

  .primary.danger {
    background: var(--d-red);
    border-color: var(--d-red);
  }

  .primary.danger:hover:not(:disabled) {
    background: #d05a54;
    border-color: #d05a54;
  }
</style>
