<script lang="ts">
  // 设置对话框(菜单小窗入口, 中文文案): 即改即存——每项落定立即写 CSS 变量并持久化,
  // 无 OK/Cancel 暂存语义; "完成"只是关闭
  import { DEFAULT_SETTINGS, settings, updateSettings } from '$lib/settings.svelte';

  let { onclose }: { onclose: () => void } = $props();

  /** 字号落定(blur/Enter): 数字化, 越界交给 Rust 钳制后回同步 */
  function commitSize(e: Event) {
    const v = Number((e.currentTarget as HTMLInputElement).value);
    if (Number.isFinite(v)) updateSettings({ editorFontSize: Math.round(v) });
  }

  function commitFamily(e: Event) {
    updateSettings({ editorFontFamily: (e.currentTarget as HTMLInputElement).value });
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal" role="dialog" aria-modal="true" aria-label="设置">
    <h3>设置</h3>
    <div class="rows">
      <label class="row">
        <span class="lbl">编辑器字号</span>
        <span class="ctl">
          <input
            class="num"
            type="number"
            min="8"
            max="32"
            value={settings.value.editorFontSize}
            onchange={commitSize}
          />
          <span class="unit dim">px</span>
        </span>
      </label>

      <label class="row">
        <span class="lbl">编辑器字体</span>
        <input
          class="txt mono"
          type="text"
          placeholder="JetBrains Mono (内嵌)"
          value={settings.value.editorFontFamily}
          onchange={commitFamily}
        />
      </label>

      <div class="row">
        <span class="lbl">关闭窗口时</span>
        <div class="seg" role="radiogroup" aria-label="关闭窗口时">
          <button
            class:on={settings.value.closeBehavior === 'tray'}
            onclick={() => updateSettings({ closeBehavior: 'tray' })}>收进托盘</button
          >
          <button
            class:on={settings.value.closeBehavior === 'quit'}
            onclick={() => updateSettings({ closeBehavior: 'quit' })}>退出应用</button
          >
        </div>
      </div>

      <label class="row">
        <span class="lbl">词级强调默认开启</span>
        <input
          type="checkbox"
          checked={settings.value.highlightWords}
          onchange={(e) =>
            updateSettings({ highlightWords: (e.currentTarget as HTMLInputElement).checked })}
        />
      </label>
    </div>
    <footer>
      <button onclick={() => updateSettings({ ...DEFAULT_SETTINGS })}>恢复默认</button>
      <span class="hint dim">即改即存 · 保存在本机</span>
      <span class="spacer"></span>
      <button class="primary" onclick={onclose}>完成</button>
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
    width: 340px;
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

  .rows {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--d-border);
    border-radius: 8px;
    background: var(--d-canvas);
    padding: 4px 10px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 34px;
    font-size: 12px;
  }

  .row + .row {
    border-top: 1px solid var(--d-border);
  }

  .lbl {
    flex: 1;
    color: var(--d-text);
  }

  .ctl {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .num,
  .txt {
    background: var(--d-panel);
    border: 1px solid var(--d-border-strong);
    border-radius: 5px;
    color: var(--d-text);
    font-size: 12px;
    padding: 3px 7px;
  }

  .num {
    width: 58px;
  }

  .txt {
    width: 168px;
  }

  .num:focus-visible,
  .txt:focus-visible {
    outline: 2px solid #3a5fae;
    outline-offset: 0;
  }

  .unit {
    font-size: 11px;
  }

  /* 双段选择(收进托盘/退出应用): 复用工具栏 .on 的选中蓝 */
  .seg {
    display: inline-flex;
    border: 1px solid var(--d-border-strong);
    border-radius: 6px;
    overflow: hidden;
  }

  .seg button {
    height: 24px;
    padding: 0 10px;
    border: none;
    border-radius: 0;
    background: var(--d-panel);
    font-size: 12px;
  }

  .seg button + button {
    border-left: 1px solid var(--d-border-strong);
  }

  .seg button.on {
    background: #243252;
    color: var(--d-sel-text);
  }

  footer {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 12px;
  }

  footer button:not(.primary) {
    background: transparent;
    border-color: var(--d-border-strong);
    color: var(--d-text);
  }

  footer button:not(.primary):hover:not(:disabled) {
    background: var(--d-hover);
  }

  .hint {
    font-size: 11px;
  }

  .spacer {
    flex: 1;
  }
</style>
