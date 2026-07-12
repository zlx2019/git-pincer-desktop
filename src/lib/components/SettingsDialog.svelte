<script lang="ts">
  // 设置对话框(菜单小窗入口, 文案随语言设置): 即改即存——每项落定立即生效并持久化,
  // 无 OK/Cancel 暂存语义; "完成"只是关闭
  import { t } from '$lib/i18n.svelte';
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
  <div class="modal" role="dialog" aria-modal="true" aria-label={t('set-title')}>
    <h3>{t('set-title')}</h3>
    <div class="rows">
      <div class="row">
        <span class="lbl">{t('set-theme')}</span>
        <div class="seg" role="radiogroup" aria-label={t('set-theme')}>
          <button
            class:on={settings.value.theme === 'dark'}
            onclick={() => updateSettings({ theme: 'dark' })}>{t('set-theme-dark')}</button
          >
          <button
            class:on={settings.value.theme === 'light'}
            onclick={() => updateSettings({ theme: 'light' })}>{t('set-theme-light')}</button
          >
        </div>
      </div>

      <div class="row">
        <span class="lbl">{t('set-lang')}</span>
        <div class="seg" role="radiogroup" aria-label={t('set-lang')}>
          <button
            class:on={settings.value.language === 'zh'}
            onclick={() => updateSettings({ language: 'zh' })}>中文</button
          >
          <button
            class:on={settings.value.language === 'en'}
            onclick={() => updateSettings({ language: 'en' })}>English</button
          >
        </div>
      </div>

      <label class="row">
        <span class="lbl">{t('set-font-size')}</span>
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
        <span class="lbl">{t('set-font-family')}</span>
        <input
          class="txt mono"
          type="text"
          placeholder={t('set-font-ph')}
          value={settings.value.editorFontFamily}
          onchange={commitFamily}
        />
      </label>

      <div class="row">
        <span class="lbl">{t('set-close')}</span>
        <div class="seg" role="radiogroup" aria-label={t('set-close')}>
          <button
            class:on={settings.value.closeBehavior === 'tray'}
            onclick={() => updateSettings({ closeBehavior: 'tray' })}>{t('set-close-tray')}</button
          >
          <button
            class:on={settings.value.closeBehavior === 'quit'}
            onclick={() => updateSettings({ closeBehavior: 'quit' })}>{t('set-close-quit')}</button
          >
        </div>
      </div>

      <label class="row">
        <span class="lbl">{t('set-words')}</span>
        <input
          type="checkbox"
          checked={settings.value.highlightWords}
          onchange={(e) =>
            updateSettings({ highlightWords: (e.currentTarget as HTMLInputElement).checked })}
        />
      </label>
    </div>
    <p class="note dim">{t('set-editor-note')}</p>
    <footer>
      <button onclick={() => updateSettings({ ...DEFAULT_SETTINGS })}>{t('set-reset')}</button>
      <span class="hint dim">{t('set-hint')}</span>
      <span class="spacer"></span>
      <button class="primary" onclick={onclose}>{t('set-done')}</button>
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
    background: var(--d-sel-dim);
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

  /* 三栏编辑器随路由新建, 相关设置非即时生效: 对话框内注明避免"没反应"误判 */
  .note {
    margin: 8px 2px 0;
    font-size: 11px;
  }

  .spacer {
    flex: 1;
  }
</style>
