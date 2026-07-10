<script lang="ts">
  // 通用选择对话框: merge/rebase 选分支(单选, 双击即确认), cherry-pick/revert 选提交(多选)
  import { SvelteSet } from 'svelte/reactivity';

  interface Item {
    id: string;
    label: string;
    sublabel?: string;
    disabled?: boolean;
  }

  let {
    title,
    items,
    multi = false,
    confirmLabel,
    onconfirm,
    onclose,
  }: {
    title: string;
    items: Item[];
    multi?: boolean;
    confirmLabel: string;
    onconfirm: (ids: string[]) => void;
    onclose: () => void;
  } = $props();

  const selected = new SvelteSet<string>();

  /** 单选替换, 多选切换 */
  function clickItem(item: Item) {
    if (item.disabled) return;
    if (multi) {
      if (selected.has(item.id)) selected.delete(item.id);
      else selected.add(item.id);
    } else {
      selected.clear();
      selected.add(item.id);
    }
  }

  /** 单选模式下双击直接确认 */
  function dblclickItem(item: Item) {
    if (item.disabled || multi) return;
    onconfirm([item.id]);
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal" role="dialog" aria-modal="true" aria-label={title}>
    <h3>{title}</h3>
    <div class="list">
      {#if items.length === 0}
        <p class="dim empty">Nothing to select.</p>
      {:else}
        {#each items as item (item.id)}
          <button
            class="item"
            class:selected={selected.has(item.id)}
            disabled={item.disabled}
            onclick={() => clickItem(item)}
            ondblclick={() => dblclickItem(item)}
          >
            <span class="label">{item.label}</span>
            {#if item.sublabel}<span class="sub dim mono">{item.sublabel}</span>{/if}
          </button>
        {/each}
      {/if}
    </div>
    <footer>
      {#if multi}<span class="dim count">{selected.size} selected</span>{/if}
      <span class="spacer"></span>
      <button onclick={onclose}>Cancel</button>
      <button class="primary" disabled={selected.size === 0} onclick={() => onconfirm([...selected])}>
        {confirmLabel}
      </button>
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
    width: 380px;
    max-width: calc(100vw - 40px);
    background: var(--d-panel);
    border: 1px solid var(--d-border-strong);
    border-radius: 10px;
    padding: 14px 16px;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
    color: var(--d-text);
  }

  h3 {
    margin: 0 0 10px;
    font-size: 13px;
    font-weight: 600;
  }

  .list {
    max-height: 300px;
    overflow: auto;
    border: 1px solid var(--d-border);
    border-radius: 8px;
    padding: 4px;
    display: flex;
    flex-direction: column;
    background: var(--d-canvas);
  }

  .empty {
    text-align: center;
    margin: 18px 0;
    color: var(--d-dim);
  }

  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: auto;
    padding: 5px 8px;
    border: none;
    background: transparent;
    border-radius: 6px;
    text-align: left;
    color: var(--d-text);
  }

  .item:hover:not(:disabled) {
    background: var(--d-hover);
  }

  .item.selected,
  .item.selected:hover {
    background: var(--d-sel);
  }

  .item .sub,
  .count {
    color: var(--d-dim);
  }

  footer button:not(.primary) {
    background: transparent;
    border-color: var(--d-border-strong);
    color: var(--d-text);
  }

  footer button:not(.primary):hover:not(:disabled) {
    background: var(--d-hover);
  }

  .item .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item .sub {
    font-size: 11px;
    flex: none;
  }

  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
  }

  .count {
    font-size: 12px;
  }

  .spacer {
    flex: 1;
  }
</style>
