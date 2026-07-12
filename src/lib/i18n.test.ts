// i18n 词典单测: 词条完整性 / 占位符对称 / 取词
import { describe, expect, it } from 'vitest';
import { dict, pick } from './i18n';

describe('i18n dict', () => {
  it('每个词条中英双份且非空', () => {
    for (const [key, entry] of Object.entries(dict)) {
      expect(entry, key).toHaveLength(2);
      expect(entry[0].length, key).toBeGreaterThan(0);
      expect(entry[1].length, key).toBeGreaterThan(0);
    }
  });

  it('占位符 {n} 两语对称(一边有另一边必须有)', () => {
    for (const [key, [zh, en]] of Object.entries(dict)) {
      expect(zh.includes('{n}'), key).toBe(en.includes('{n}'));
    }
  });

  it('pick 按语言取词并代入占位符', () => {
    expect(pick('set-title', 'zh')).toBe('设置');
    expect(pick('set-title', 'en')).toBe('Settings');
    expect(pick('resume-pending', 'zh', 3)).toBe('3 个冲突待解决 · 点击恢复');
    expect(pick('term-done', 'en', '1.2')).toBe('✔ done · 1.2s');
    expect(pick('sb-oping', 'en', 'merge')).toBe('● merge in progress');
  });
});
