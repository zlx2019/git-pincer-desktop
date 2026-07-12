import { describe, expect, it } from 'vitest';
import { fileIcon } from './fileicon';

describe('fileIcon', () => {
  it('maps known extensions to letter badges with category colors', () => {
    const rs = fileIcon('src/main.rs');
    expect(rs.svg).toContain('>RS</text>');
    expect(rs.color).toBe('var(--d-amber)');
    expect(fileIcon('a/b/Cargo.lock').svg).toContain('>LK</text>');
  });

  it('maps image extensions to the thumbnail glyph', () => {
    expect(fileIcon('assets/logo.PNG').svg).toContain('circle');
  });

  it('falls back to the generic document icon', () => {
    const doc = fileIcon('Makefile');
    expect(doc.svg).toContain('M3.5 1.5h6l3 3v10h-9z');
    expect(doc.color).toBe('var(--d-dim)');
    expect(fileIcon('weird.xyz').svg).toBe(doc.svg);
    // 隐藏文件的前导点不算扩展名分隔符
    expect(fileIcon('.gitignore').svg).toBe(doc.svg);
  });
});
