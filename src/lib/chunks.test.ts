// chunks.ts 纯逻辑单测: 状态机 / 导航回绕 / 批量应用过滤 / 文本组装 / 区间换算
import { Text } from '@codemirror/state';
import { describe, expect, it } from 'vitest';
import type { ChunkKind, ChunkVisual, MergeChunk } from './api';
import {
  applyAllTargets,
  applyEdit,
  isResolved,
  joinedText,
  lineRangeToPos,
  navTarget,
  paneClass,
  paneRange,
  relevantSides,
  type ChunkState,
} from './chunks';

/** 造 chunk: 三栏区间同值(测试不关心区间错位) */
function ck(id: number, kind: ChunkKind, visual: ChunkVisual = 'modified'): MergeChunk {
  const r: [number, number] = [id, id + 1];
  return {
    id,
    kind,
    visual,
    leftRange: r,
    resultRange: r,
    rightRange: r,
    leftEmphasis: [],
    rightEmphasis: [],
  };
}

/** 造状态(默认双侧 pending 未编辑) */
function st(over: Partial<ChunkState> = {}): ChunkState {
  return { ours: 'pending', theirs: 'pending', edited: false, ...over };
}

describe('relevantSides / isResolved', () => {
  it('单侧与 agree 只需处理 ours/theirs 一侧, conflict 双侧', () => {
    expect(relevantSides(ck(0, 'ours'))).toEqual(['ours']);
    expect(relevantSides(ck(0, 'agree'))).toEqual(['ours']);
    expect(relevantSides(ck(0, 'theirs'))).toEqual(['theirs']);
    expect(relevantSides(ck(0, 'conflict'))).toEqual(['ours', 'theirs']);
  });

  it('相关侧全部处理(applied 或 ignored)才算解决', () => {
    expect(isResolved(ck(0, 'ours'), st())).toBe(false);
    expect(isResolved(ck(0, 'ours'), st({ ours: 'applied' }))).toBe(true);
    expect(isResolved(ck(0, 'ours'), st({ ours: 'ignored' }))).toBe(true);
    // conflict 单侧处理不算完
    expect(isResolved(ck(0, 'conflict'), st({ ours: 'applied' }))).toBe(false);
    expect(isResolved(ck(0, 'conflict'), st({ ours: 'applied', theirs: 'ignored' }))).toBe(true);
    // theirs chunk 与 ours 侧无关
    expect(isResolved(ck(0, 'theirs'), st({ theirs: 'applied' }))).toBe(true);
  });

  it('手工编辑直接短路为已解决; 上下文缺失恒为未解决', () => {
    expect(isResolved(ck(0, 'conflict'), st({ edited: true }))).toBe(true);
    expect(isResolved(undefined, st())).toBe(false);
    expect(isResolved(ck(0, 'ours'), undefined)).toBe(false);
  });
});

describe('paneClass', () => {
  it('对侧无关的栏不着色', () => {
    expect(paneClass(ck(0, 'theirs'), st(), 'left')).toBeNull();
    expect(paneClass(ck(0, 'ours'), st(), 'right')).toBeNull();
    expect(paneClass(ck(0, 'agree'), st(), 'left')).toBe('ck-modified');
    expect(paneClass(ck(0, 'agree'), st(), 'right')).toBe('ck-modified');
  });

  it('pending 用形态色, 该侧已处理或手工编辑后降为 ck-done', () => {
    expect(paneClass(ck(0, 'conflict', 'conflict'), st(), 'left')).toBe('ck-conflict');
    expect(paneClass(ck(0, 'ours'), st({ ours: 'ignored' }), 'left')).toBe('ck-done');
    expect(paneClass(ck(0, 'conflict'), st({ theirs: 'applied' }), 'right')).toBe('ck-done');
    // conflict 只处理了 theirs → 左栏仍是形态色
    expect(paneClass(ck(0, 'conflict', 'conflict'), st({ theirs: 'applied' }), 'left')).toBe(
      'ck-conflict'
    );
    expect(paneClass(ck(0, 'ours'), st({ edited: true }), 'left')).toBe('ck-done');
  });
});

describe('navTarget', () => {
  it('正向取下一个更大的 id, 越界回绕到首个', () => {
    const pool = [1, 3, 5, 7];
    expect(navTarget(pool, -1, 1)).toBe(1);
    expect(navTarget(pool, 1, 1)).toBe(3);
    expect(navTarget(pool, 7, 1)).toBe(1);
    expect(navTarget(pool, 4, 1)).toBe(5); // cur 不在 pool 内也能落位
  });

  it('反向取上一个更小的 id, 越界回绕到末个(⇧F7 回归)', () => {
    const pool = [1, 3, 5, 7];
    // 旧实现 findIndex(i < cur) 恒取首个, 会 7→1→7 乒乓; 正确序列是 7→5→3→1→7
    expect(navTarget(pool, 7, -1)).toBe(5);
    expect(navTarget(pool, 5, -1)).toBe(3);
    expect(navTarget(pool, 3, -1)).toBe(1);
    expect(navTarget(pool, 1, -1)).toBe(7);
  });
});

describe('applyAllTargets', () => {
  const chunks = [ck(0, 'ours'), ck(1, 'agree'), ck(2, 'theirs'), ck(3, 'conflict')];
  const fresh = () => chunks.map(() => st());

  it('left 应用 ours+agree, right 应用 theirs+agree(对称), conflict 永不批量', () => {
    expect(applyAllTargets(chunks, fresh(), 'left')).toEqual([
      { id: 0, side: 'ours' },
      { id: 1, side: 'ours' },
    ]);
    expect(applyAllTargets(chunks, fresh(), 'right')).toEqual([
      { id: 1, side: 'ours' },
      { id: 2, side: 'theirs' },
    ]);
    expect(applyAllTargets(chunks, fresh(), 'all')).toHaveLength(3);
  });

  it('已处理或手工编辑过的 chunk 跳过', () => {
    const states = fresh();
    states[0].ours = 'ignored';
    states[1].edited = true;
    expect(applyAllTargets(chunks, states, 'all')).toEqual([{ id: 2, side: 'theirs' }]);
  });
});

describe('joinedText / applyEdit', () => {
  it('区间尾不在文末补尾换行, 在文末不补; 空行集为空串', () => {
    expect(joinedText(['a', 'b'], 5, 100)).toBe('a\nb\n');
    expect(joinedText(['a', 'b'], 100, 100)).toBe('a\nb');
    expect(joinedText([], 5, 100)).toBe('');
  });

  it('首侧应用 = 区间替换', () => {
    expect(applyEdit(['x'], { from: 4, to: 6 }, false, 100)).toEqual({
      from: 4,
      to: 6,
      insert: 'x\n',
    });
  });

  it('第二侧应用 = 追加到区间尾(keep both); 区间尾即文末时前补换行防同行粘连', () => {
    expect(applyEdit(['y'], { from: 4, to: 6 }, true, 100)).toEqual({
      from: 6,
      to: 6,
      insert: 'y\n',
    });
    expect(applyEdit(['y'], { from: 90, to: 100 }, true, 100)).toEqual({
      from: 100,
      to: 100,
      insert: '\ny',
    });
  });
});

describe('paneRange / lineRangeToPos', () => {
  it('单侧改动的对侧区间为 null', () => {
    expect(paneRange(ck(0, 'theirs'), 'left')).toBeNull();
    expect(paneRange(ck(0, 'ours'), 'right')).toBeNull();
    expect(paneRange(ck(0, 'conflict'), 'result')).toEqual([0, 1]);
  });

  it('行区间 → 位置区间: 半开、空区间、文末封顶', () => {
    const doc = Text.of(['a', 'b', 'c', '']); // "a\nb\nc\n", 4 行, 长度 6
    expect(lineRangeToPos(doc, [0, 1])).toEqual({ from: 0, to: 2 });
    expect(lineRangeToPos(doc, [1, 3])).toEqual({ from: 2, to: 6 });
    expect(lineRangeToPos(doc, [3, 3])).toEqual({ from: 6, to: 6 }); // 空区间(插入点)
    expect(lineRangeToPos(doc, [2, 4])).toEqual({ from: 4, to: 6 }); // 区间尾越界取文末
  });
});
