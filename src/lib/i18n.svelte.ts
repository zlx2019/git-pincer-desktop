// 响应式取词: 在模板/derived 里调用即建立对语言设置的依赖,
// 切换语言后全部已挂载 UI 即时刷新, 无需重载
import { pick, type I18nKey } from './i18n';
import { settings } from './settings.svelte';

/** 按当前语言取词(可带 {n} 占位参数) */
export function t(key: I18nKey, n?: string | number): string {
  return pick(key, settings.value.language, n);
}
