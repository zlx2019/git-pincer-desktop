<script lang="ts">
  // 二进制文件 pick-one 对话框: 二进制无法三方合并, 只能整体取一侧
  import { api } from '$lib/api';
  import { toast } from '$lib/toast.svelte';

  let {
    path,
    onclose,
    onresolved,
  }: { path: string; onclose: () => void; onresolved: () => void } = $props();

  let busy = $state(false);

  /** 取一侧并通知父级刷新 */
  async function accept(side: 'yours' | 'theirs') {
    busy = true;
    try {
      await api.acceptSide([path], side);
      onresolved();
    } catch (e) {
      toast(String(e));
      busy = false;
    }
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal" role="dialog" aria-modal="true" aria-label="Resolve binary file">
    <h3>Binary File</h3>
    <p class="mono path">{path}</p>
    <p class="dim">Binary files cannot be merged — choose which version to keep.</p>
    <footer>
      <button disabled={busy} onclick={onclose}>Cancel</button>
      <span class="spacer"></span>
      <button disabled={busy} onclick={() => accept('yours')}>Accept Yours</button>
      <button class="primary" disabled={busy} onclick={() => accept('theirs')}>Accept Theirs</button>
    </footer>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
  }

  .modal {
    width: 460px;
    background: var(--d-panel);
    border: 1px solid var(--d-border-strong);
    border-radius: 10px;
    padding: 18px 20px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
    color: var(--d-text);
  }

  h3 {
    margin: 0 0 10px;
    font-size: 15px;
  }

  .path {
    font-size: 12px;
    margin: 0 0 6px;
    -webkit-user-select: text;
    user-select: text;
  }

  footer {
    display: flex;
    gap: 8px;
    margin-top: 18px;
  }

  .spacer {
    flex: 1;
  }
</style>
