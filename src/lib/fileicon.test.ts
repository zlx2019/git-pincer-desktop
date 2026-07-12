import { describe, expect, it } from 'vitest';
import { fileIcon } from './fileicon';

describe('fileIcon', () => {
  it('maps known extensions to Atom file-icons glyphs with category colors', () => {
    const rs = fileIcon('src/main.rs');
    expect(rs.svg).toContain('viewBox="0 0 512 512"');
    expect(rs.svg).toContain('fill="currentColor"');
    expect(rs.color).toBe('var(--d-amber)');
    // yaml glyph 是 polygon 源
    expect(fileIcon('ci.yml').svg).toContain('<polygon');
    expect(fileIcon('a/b.svelte').color).toBe('var(--d-red)');
  });

  it('recognizes git metafiles by name', () => {
    expect(fileIcon('.gitignore').svg).toBe(fileIcon('sub/.gitattributes').svg);
    expect(fileIcon('.gitignore').color).toBe('var(--d-red)');
  });

  it('keeps letter badges for types without a glyph', () => {
    expect(fileIcon('Cargo.lock').svg).toContain('>LK</text>');
  });

  it('maps image extensions to the thumbnail glyph', () => {
    expect(fileIcon('assets/logo.PNG').svg).toContain('circle');
  });

  it('falls back to the generic document icon', () => {
    const doc = fileIcon('Makefile');
    expect(doc.svg).toContain('M3.5 1.5h6l3 3v10h-9z');
    expect(doc.color).toBe('var(--d-dim)');
    expect(fileIcon('weird.xyz').svg).toBe(doc.svg);
  });
});
