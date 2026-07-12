<script lang="ts">
  // 设置对话框(菜单小窗入口, 文案随语言设置): 即改即存——每项落定立即生效并持久化,
  // 无 OK/Cancel 暂存语义; "完成"只是关闭。
  // 布局分三页签: 通用(语言/关窗) · 界面(主题/编辑器) · 关于; 正文区定高, 切页不跳动
  import { getVersion } from '@tauri-apps/api/app';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { t } from '$lib/i18n.svelte';
  import { DEFAULT_SETTINGS, EMBEDDED_FONTS, settings, updateSettings } from '$lib/settings.svelte';

  let { onclose }: { onclose: () => void } = $props();

  const TABS = ['general', 'ui', 'about'] as const;
  let tab = $state<(typeof TABS)[number]>('general');

  // 关于页的版本号(壳层查询; mock/测试环境拿不到就不显示)
  let version = $state('');
  getVersion()
    .then((v) => (version = v ?? ''))
    .catch(() => {});

  /** 字号落定(blur/Enter): 数字化, 越界交给 Rust 钳制后回同步 */
  function commitSize(e: Event) {
    const v = Number((e.currentTarget as HTMLInputElement).value);
    if (Number.isFinite(v)) updateSettings({ editorFontSize: Math.round(v) });
  }

  /** 当前存储值是否自定义字体(非空且不在内嵌清单; '' = 默认 JetBrains Mono) */
  function isCustomFamily(f: string): boolean {
    const fam = f.trim();
    return fam !== '' && !EMBEDDED_FONTS.includes(fam as (typeof EMBEDDED_FONTS)[number]);
  }

  // 选中"自定义…"但尚未输入时输入框也要保持展开, 不能只从存储值推导
  let customFont = $state(isCustomFamily(settings.value.editorFontFamily));
  const fontChoice = $derived(
    customFont || isCustomFamily(settings.value.editorFontFamily)
      ? '__custom'
      : settings.value.editorFontFamily.trim()
  );

  function pickFamily(e: Event) {
    const v = (e.currentTarget as HTMLSelectElement).value;
    if (v === '__custom') {
      customFont = true;
      return;
    }
    customFont = false;
    updateSettings({ editorFontFamily: v });
  }

  function commitFamily(e: Event) {
    updateSettings({ editorFontFamily: (e.currentTarget as HTMLInputElement).value });
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && onclose()} />

<div class="overlay" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div class="modal" role="dialog" aria-modal="true" aria-label={t('set-title')}>
    <h3>{t('set-title')}</h3>

    <div class="tabs" role="tablist">
      {#each TABS as tb (tb)}
        <button
          class="tab"
          class:on={tab === tb}
          role="tab"
          aria-selected={tab === tb}
          onclick={() => (tab = tb)}>{t(`set-tab-${tb}`)}</button
        >
      {/each}
    </div>

    <div class="body">
      {#if tab === 'general'}
        <div class="rows">
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

          <div class="row">
            <span class="lbl">{t('set-close')}</span>
            <div class="seg" role="radiogroup" aria-label={t('set-close')}>
              <button
                class:on={settings.value.closeBehavior === 'tray'}
                onclick={() => updateSettings({ closeBehavior: 'tray' })}
                >{t('set-close-tray')}</button
              >
              <button
                class:on={settings.value.closeBehavior === 'quit'}
                onclick={() => updateSettings({ closeBehavior: 'quit' })}
                >{t('set-close-quit')}</button
              >
            </div>
          </div>
        </div>
      {:else if tab === 'ui'}
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
            <span class="selwrap">
              <select class="txt sel" value={fontChoice} onchange={pickFamily}>
                <option value="">{t('set-font-emb', EMBEDDED_FONTS[0])}</option>
                {#each EMBEDDED_FONTS.slice(1) as f (f)}
                  <option value={f}>{t('set-font-emb', f)}</option>
                {/each}
                <option value="__custom">{t('set-font-custom')}</option>
              </select>
            </span>
          </label>

          {#if fontChoice === '__custom'}
            <label class="row">
              <span class="lbl"></span>
              <input
                class="txt mono"
                type="text"
                placeholder={t('set-font-ph')}
                value={isCustomFamily(settings.value.editorFontFamily)
                  ? settings.value.editorFontFamily
                  : ''}
                onchange={commitFamily}
              />
            </label>
          {/if}

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
      {:else}
        <div class="about">
          <div class="brand">PINCER</div>
          <div class="aver dim">git-pincer-desktop{version ? ` · v${version}` : ''}</div>
          <button
            class="link"
            onclick={() => openUrl('https://github.com/zlx2019/git-pincer-desktop').catch(() => {})}
            >{t('set-github')}</button
          >
          <p class="credits dim">{t('set-about-lic')}</p>
          <p class="credits dim">
            {t('set-about-thanks')}similar · CodeMirror 6 · JetBrains Mono · Maple Mono ·
            file-icons
          </p>
        </div>
      {/if}
    </div>

    <footer>
      {#if tab !== 'about'}
        <button
          onclick={() => {
            customFont = false;
            updateSettings({ ...DEFAULT_SETTINGS });
          }}>{t('set-reset')}</button
        >
        <span class="hint dim">{t('set-hint')}</span>
      {/if}
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
    margin: 0 0 8px;
    font-size: 13px;
    font-weight: 600;
  }

  /* 页签条: 活动态橙色下划线(用色约定允许的"活动 tab 下划线", 与终端 tab 同语言) */
  .tabs {
    display: flex;
    gap: 2px;
    margin-bottom: 10px;
    border-bottom: 1px solid var(--d-border);
  }

  .tab {
    position: relative;
    height: 26px;
    padding: 0 10px;
    border: none;
    border-radius: 0;
    background: none;
    font-size: 12px;
    color: var(--d-dim);
  }

  .tab:hover {
    color: var(--d-text);
    background: none;
  }

  .tab.on {
    color: var(--d-text);
  }

  .tab.on::after {
    content: '';
    position: absolute;
    left: 6px;
    right: 6px;
    bottom: -1px;
    height: 2px;
    background: var(--d-orange);
  }

  /* 正文定高: 三个页签内容高度不同, 切换时对话框不跳动 */
  .body {
    min-height: 176px;
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

  /* 字体下拉: 去掉原生外观自绘箭头, 与 .txt 输入框同一视觉 */
  .selwrap {
    position: relative;
    display: inline-flex;
  }

  .sel {
    appearance: none;
    -webkit-appearance: none;
    width: 168px;
    padding-right: 22px;
  }

  .selwrap::after {
    content: '';
    position: absolute;
    right: 9px;
    top: 50%;
    margin-top: -2px;
    border-left: 4px solid transparent;
    border-right: 4px solid transparent;
    border-top: 5px solid var(--d-dim);
    pointer-events: none;
  }

  .num:focus-visible,
  .txt:focus-visible {
    outline: 2px solid #3a5fae;
    outline-offset: 0;
  }

  .unit {
    font-size: 11px;
  }

  /* 双段选择: 复用工具栏 .on 的选中蓝 */
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

  /* 三栏编辑器随路由新建, 相关设置非即时生效: 界面页签内注明避免"没反应"误判 */
  .note {
    margin: 8px 2px 0;
    font-size: 11px;
  }

  /* 关于页签: 居中品牌块(品牌橙仅 logo 字样, 遵守用色约定) */
  .about {
    min-height: 176px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    text-align: center;
  }

  .brand {
    color: var(--d-orange);
    font-weight: 700;
    letter-spacing: 0.18em;
    font-size: 16px;
  }

  .aver {
    font-size: 11px;
  }

  .link {
    background: none;
    border: none;
    padding: 0;
    height: auto;
    font-size: 11px;
    color: var(--d-blue-lt);
    cursor: pointer;
  }

  .link:hover {
    text-decoration: underline;
    background: none;
  }

  .credits {
    margin: 0;
    font-size: 11px;
    max-width: 260px;
  }

  .spacer {
    flex: 1;
  }
</style>
