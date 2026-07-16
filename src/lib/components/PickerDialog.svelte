<script lang="ts">
  // 通用选择对话框: merge/rebase 选分支(单选, 双击即确认), cherry-pick/revert 选提交(多选)
  import { SvelteSet } from 'svelte/reactivity';

  interface Item {
    id: string;
    label: string;
    sublabel?: string;
    /** 附加小标签(如提交的来源分支), 以描边 chip 呈现 */
    tag?: string;
    disabled?: boolean;
  }

  let {
    title,
    items,
    multi = false,
    loading = false,
    confirmLabel,
    onconfirm,
    onclose,
  }: {
    title: string;
    items: Item[];
    multi?: boolean;
    /** 乐观弹窗: 列表数据仍在拉取中, 显示占位转圈 */
    loading?: boolean;
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

  // 乐观弹窗会在双击的两击之间挂载, 第二击正落在刚出现的遮罩上;
  // 忽略挂载后 250ms(双击间隔量级)内的遮罩点击, 防对话框被瞬间误关
  const openedAt = performance.now();

  /** 点遮罩空白处关闭(带误触守卫) */
  function overlayClick(e: MouseEvent) {
    if (e.target !== e.currentTarget) return;
    if (performance.now() - openedAt < 250) return;
    onclose();
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="overlay" role="presentation" onclick={overlayClick}>
  <div class="modal" role="dialog" aria-modal="true" aria-label={title}>
    <h3>{title}</h3>
    <div class="list">
      {#if loading}
        <p class="empty"><span class="spin"></span></p>
      {:else if items.length === 0}
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
            {#if item.tag}<span class="tag mono">{item.tag}</span>{/if}
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
  /* 入场动效只动 opacity/transform(合成器属性), 时长克制不挡交互 */
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.35);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
    animation: overlay-in 0.12s ease-out;
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
    font-size: 13px;
    font-weight: 600;
  }

  .list {
    /* 最小高度垫住 loading→数据的回填: 少量条目/转圈占位不再引起模态框高度突跳 */
    min-height: 140px;
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
    /* flex 列容器里上下 auto margin = 在 min-height 撑起的列表里垂直居中 */
    text-align: center;
    margin: auto 0;
    color: var(--d-dim);
  }

  /* 数据加载中的占位转圈(乐观弹窗: 先弹后取数)。延迟 0.15s 渐显——
     本地 git 常在百毫秒内返回, 快路径下只见稳定空列表, 不见转圈一闪而过 */
  .spin {
    display: inline-block;
    width: 14px;
    height: 14px;
    border: 2px solid var(--d-spin-track);
    border-top-color: var(--d-blue-lt);
    border-radius: 50%;
    animation:
      spin 0.8s linear infinite,
      spin-appear 0.15s ease-out 0.15s backwards;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes spin-appear {
    from {
      opacity: 0;
    }
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
    transition: background-color 0.1s ease-out;
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

  .item .tag {
    flex: none;
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 10px;
    line-height: 15px;
    color: var(--d-dim);
    border: 1px solid var(--d-border-strong);
    border-radius: 4px;
    padding: 0 5px;
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
