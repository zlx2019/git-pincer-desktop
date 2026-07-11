//! Three-way merge chunking engine: two line-level diffs against the base
//! (Myers via `similar`, deadline-guarded) are grouped by base-range
//! collision. Pure logic — no git, no UI. Deliberately conservative:
//! one conflict too many beats a silently wrong merge.

use std::ops::Range;
use std::time::{Duration, Instant};

use serde::Serialize;
use similar::{Algorithm, DiffTag, capture_diff_slices, capture_diff_slices_deadline};

/// 行级 diff 时间上限; 超时后 similar 降级输出粗粒度结果
const DIFF_DEADLINE: Duration = Duration::from_millis(500);
/// 任一侧超过该字节数直接降级为整文件单冲突块
const MAX_DIFF_BYTES: usize = 2 * 1024 * 1024;
/// 词级强调只处理行数不超过该值的 chunk(控制成本)
const MAX_EMPHASIS_LINES: usize = 200;
/// 词级强调只比较字符数不超过该值的行(字符级 diff 无 deadline, 超长单行代价失控)
const MAX_EMPHASIS_LINE_CHARS: usize = 1000;

/// chunk 的来源侧分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChunkKind {
    /// 仅我方改动
    Ours,
    /// 仅对方改动
    Theirs,
    /// 双方改动且内容一致
    Agree,
    /// 双方改动且不一致
    Conflict,
}

/// 着色形态(决定 UI 底色)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChunkVisual {
    /// 新增(base 侧为空)
    Added,
    /// 删除(改动侧为空)
    Deleted,
    /// 修改
    Modified,
    /// 冲突
    Conflict,
}

/// 快照中的一个 chunk; 行区间 [start, end) 与 CM6 行模型一致(全文按 \n 切分)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkView {
    /// 序号(自 0 递增, base 顺序)
    pub id: usize,
    /// 来源侧
    pub kind: ChunkKind,
    /// 着色形态
    pub visual: ChunkVisual,
    /// 左栏(ours)行区间
    pub left_range: [usize; 2],
    /// 中栏(result, 初始为 base)行区间
    pub result_range: [usize; 2],
    /// 右栏(theirs)行区间
    pub right_range: [usize; 2],
    /// 左栏词级强调: [chunk 内行偏移, UTF-16 起, UTF-16 止]
    pub left_emphasis: Vec<[u32; 3]>,
    /// 右栏词级强调
    pub right_emphasis: Vec<[u32; 3]>,
}

/// 三栏快照
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeSnapshot {
    /// 文件路径(相对仓库根)
    pub path: String,
    /// 左栏全文(ours)
    pub left: String,
    /// 中栏初始全文(base)
    pub result: String,
    /// 右栏全文(theirs)
    pub right: String,
    /// 全部 chunk(base 顺序)
    pub chunks: Vec<ChunkView>,
    /// change 总数
    pub changes: usize,
    /// 冲突数
    pub conflicts: usize,
}

/// 一段非 Equal 的 diff 区间: base 行区间 → 该侧行区间
#[derive(Debug, Clone)]
struct Seg {
    base: Range<usize>,
    side: Range<usize>,
}

/// 一个碰撞簇: 双方 seg 按 base 区间相触归并
struct Cluster {
    start: usize,
    end: usize,
    a: Vec<Seg>,
    b: Vec<Seg>,
}

/// 构建三栏快照(入口, 纯函数)
pub fn build_snapshot(path: &str, base: &str, ours: &str, theirs: &str) -> MergeSnapshot {
    let b: Vec<&str> = base.split('\n').collect();
    let o: Vec<&str> = ours.split('\n').collect();
    let t: Vec<&str> = theirs.split('\n').collect();

    let oversized =
        base.len() > MAX_DIFF_BYTES || ours.len() > MAX_DIFF_BYTES || theirs.len() > MAX_DIFF_BYTES;
    let chunks = if oversized {
        vec![whole_file_conflict(&b, &o, &t)]
    } else {
        let deadline = Instant::now() + DIFF_DEADLINE;
        let a_segs = segments(&b, &o, deadline);
        let b_segs = segments(&b, &t, deadline);
        build_chunks(&b, &o, &t, &a_segs, &b_segs)
    };

    let changes = chunks.len();
    let conflicts = chunks
        .iter()
        .filter(|c| c.kind == ChunkKind::Conflict)
        .count();
    MergeSnapshot {
        path: path.to_owned(),
        left: ours.to_owned(),
        result: base.to_owned(),
        right: theirs.to_owned(),
        chunks,
        changes,
        conflicts,
    }
}

/// 全文降级: 单个覆盖三栏全部行的冲突块
fn whole_file_conflict(b: &[&str], o: &[&str], t: &[&str]) -> ChunkView {
    ChunkView {
        id: 0,
        kind: ChunkKind::Conflict,
        visual: ChunkVisual::Conflict,
        left_range: [0, o.len()],
        result_range: [0, b.len()],
        right_range: [0, t.len()],
        left_emphasis: Vec::new(),
        right_emphasis: Vec::new(),
    }
}

/// 行级 diff → 非 Equal 区间列表(base 顺序)
fn segments(base: &[&str], side: &[&str], deadline: Instant) -> Vec<Seg> {
    capture_diff_slices_deadline(Algorithm::Myers, base, side, Some(deadline))
        .into_iter()
        .filter(|op| op.tag() != DiffTag::Equal)
        .map(|op| Seg {
            base: op.old_range(),
            side: op.new_range(),
        })
        .collect()
}

/// 归并两侧 seg 为碰撞簇: base 区间相触(含端点)即归并, 刻意保守
fn clusters(a_segs: &[Seg], b_segs: &[Seg]) -> Vec<Cluster> {
    let mut out: Vec<Cluster> = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < a_segs.len() || j < b_segs.len() {
        // 以 base 起点更靠前的 seg 开簇
        let take_a = match (a_segs.get(i), b_segs.get(j)) {
            (Some(x), Some(y)) => x.base.start <= y.base.start,
            (Some(_), None) => true,
            _ => false,
        };
        let first = if take_a {
            i += 1;
            a_segs[i - 1].clone()
        } else {
            j += 1;
            b_segs[j - 1].clone()
        };
        let mut c = Cluster {
            start: first.base.start,
            end: first.base.end,
            a: Vec::new(),
            b: Vec::new(),
        };
        if take_a {
            c.a.push(first);
        } else {
            c.b.push(first);
        }
        // 吸收所有与簇区间相触的后续 seg(两侧交替尝试直到都不再增长)
        loop {
            let mut grew = false;
            if let Some(s) = a_segs.get(i)
                && s.base.start <= c.end
            {
                c.end = c.end.max(s.base.end);
                c.a.push(s.clone());
                i += 1;
                grew = true;
            }
            if let Some(s) = b_segs.get(j)
                && s.base.start <= c.end
            {
                c.end = c.end.max(s.base.end);
                c.b.push(s.clone());
                j += 1;
                grew = true;
            }
            if !grew {
                break;
            }
        }
        out.push(c);
    }
    out
}

/// 由簇构建 chunk: 每侧区间 = 簇前累计对齐偏移 + 簇内 覆盖/插入 折算
fn build_chunks(
    b: &[&str],
    o: &[&str],
    t: &[&str],
    a_segs: &[Seg],
    b_segs: &[Seg],
) -> Vec<ChunkView> {
    let mut out = Vec::new();
    // 该侧相对 base 的累计行偏移(当前簇之前)
    let mut delta_a: isize = 0;
    let mut delta_b: isize = 0;
    for (id, c) in clusters(a_segs, b_segs).into_iter().enumerate() {
        let span = c.end - c.start;
        let side_range = |segs: &[Seg], delta: isize| -> Range<usize> {
            let covered: usize = segs.iter().map(|s| s.base.len()).sum();
            let inserted: usize = segs.iter().map(|s| s.side.len()).sum();
            let start = usize::try_from(c.start as isize + delta).unwrap_or(0);
            start..start + span - covered + inserted
        };
        let ours_r = side_range(&c.a, delta_a);
        let theirs_r = side_range(&c.b, delta_b);
        delta_a += ours_r.len() as isize - span as isize;
        delta_b += theirs_r.len() as isize - span as isize;

        let base_lines = &b[c.start..c.end];
        let ours_lines = &o[ours_r.clone()];
        let theirs_lines = &t[theirs_r.clone()];
        let kind = match (!c.a.is_empty(), !c.b.is_empty()) {
            (true, true) => {
                if ours_lines == theirs_lines {
                    ChunkKind::Agree
                } else {
                    ChunkKind::Conflict
                }
            }
            (true, false) => ChunkKind::Ours,
            (false, true) => ChunkKind::Theirs,
            (false, false) => continue,
        };
        let visual = if kind == ChunkKind::Conflict {
            ChunkVisual::Conflict
        } else {
            let side_len = if kind == ChunkKind::Theirs {
                theirs_r.len()
            } else {
                ours_r.len()
            };
            if side_len == 0 {
                ChunkVisual::Deleted
            } else if span == 0 {
                ChunkVisual::Added
            } else {
                ChunkVisual::Modified
            }
        };
        let (left_emphasis, right_emphasis) = emphasis(kind, base_lines, ours_lines, theirs_lines);
        out.push(ChunkView {
            id,
            kind,
            visual,
            left_range: [ours_r.start, ours_r.end],
            result_range: [c.start, c.end],
            right_range: [theirs_r.start, theirs_r.end],
            left_emphasis,
            right_emphasis,
        });
    }
    out
}

/// 词级强调: Conflict 比 ours↔theirs, 单侧/一致比 该侧↔base
fn emphasis(
    kind: ChunkKind,
    base: &[&str],
    ours: &[&str],
    theirs: &[&str],
) -> (Vec<[u32; 3]>, Vec<[u32; 3]>) {
    let mut left = Vec::new();
    let mut right = Vec::new();
    match kind {
        ChunkKind::Conflict => {
            if ours.len().max(theirs.len()) <= MAX_EMPHASIS_LINES {
                for (i, (a, b)) in ours.iter().zip(theirs.iter()).enumerate() {
                    let (ea, eb) = line_emphasis(a, b);
                    push_ranges(&mut left, i, ea);
                    push_ranges(&mut right, i, eb);
                }
            }
        }
        ChunkKind::Ours | ChunkKind::Agree => side_emphasis(base, ours, &mut left),
        ChunkKind::Theirs => side_emphasis(base, theirs, &mut right),
    }
    (left, right)
}

/// 单侧相对 base 的逐行强调(行数不匹配的尾部跳过)
fn side_emphasis(base: &[&str], side: &[&str], out: &mut Vec<[u32; 3]>) {
    if side.len().max(base.len()) > MAX_EMPHASIS_LINES {
        return;
    }
    for (i, (b, s)) in base.iter().zip(side.iter()).enumerate() {
        let (_, es) = line_emphasis(b, s);
        push_ranges(out, i, es);
    }
}

/// 收集一行的强调区间
fn push_ranges(out: &mut Vec<[u32; 3]>, line: usize, ranges: Vec<(u32, u32)>) {
    for (from, to) in ranges {
        out.push([line as u32, from, to]);
    }
}

/// 一行内的强调区间列表(UTF-16 起止)
type LineRanges = Vec<(u32, u32)>;

/// 两行的字符级 diff → 各自的 UTF-16 偏移差异区间(与 CM6 文档坐标一致);
/// 任一行超过 MAX_EMPHASIS_LINE_CHARS 时跳过(该行无强调, chunk 底色不受影响)
fn line_emphasis(a: &str, b: &str) -> (LineRanges, LineRanges) {
    if a == b || too_long(a) || too_long(b) {
        return (Vec::new(), Vec::new());
    }
    let ac: Vec<char> = a.chars().collect();
    let bc: Vec<char> = b.chars().collect();
    let ops = capture_diff_slices(Algorithm::Myers, &ac, &bc);
    let mut ra = Vec::new();
    let mut rb = Vec::new();
    for op in ops {
        if op.tag() == DiffTag::Equal {
            continue;
        }
        append_range(&mut ra, char_range_to_utf16(&ac, op.old_range()));
        append_range(&mut rb, char_range_to_utf16(&bc, op.new_range()));
    }
    (ra, rb)
}

/// 行内字符数是否超过词级强调上限(只走前 上限+1 个字符, 不遍历全行)
fn too_long(s: &str) -> bool {
    s.chars().nth(MAX_EMPHASIS_LINE_CHARS).is_some()
}

/// char 索引区间 → UTF-16 偏移区间(空区间丢弃)
fn char_range_to_utf16(chars: &[char], r: Range<usize>) -> Option<(u32, u32)> {
    if r.is_empty() {
        return None;
    }
    let from: usize = chars[..r.start].iter().map(|c| c.len_utf16()).sum();
    let len: usize = chars[r.clone()].iter().map(|c| c.len_utf16()).sum();
    Some((from as u32, (from + len) as u32))
}

/// 追加区间, 与上一个相接则合并(降低碎片)
fn append_range(out: &mut Vec<(u32, u32)>, r: Option<(u32, u32)>) {
    let Some((from, to)) = r else { return };
    if let Some(last) = out.last_mut()
        && last.1 >= from
    {
        last.1 = last.1.max(to);
        return;
    }
    out.push((from, to));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(base: &str, ours: &str, theirs: &str) -> MergeSnapshot {
        build_snapshot("t.txt", base, ours, theirs)
    }

    #[test]
    fn ours_only_modification() {
        let s = snap("a\nb\nc\n", "a\nB\nc\n", "a\nb\nc\n");
        assert_eq!(s.chunks.len(), 1);
        let c = &s.chunks[0];
        assert_eq!(c.kind, ChunkKind::Ours);
        assert_eq!(c.visual, ChunkVisual::Modified);
        assert_eq!(c.left_range, [1, 2]);
        assert_eq!(c.result_range, [1, 2]);
        assert_eq!(c.right_range, [1, 2]);
        assert_eq!(s.conflicts, 0);
    }

    #[test]
    fn non_overlapping_changes_stay_separate() {
        let s = snap("a\nb\nc\nd\ne\n", "A\nb\nc\nd\ne\n", "a\nb\nc\nd\nE\n");
        assert_eq!(s.chunks.len(), 2);
        assert_eq!(s.chunks[0].kind, ChunkKind::Ours);
        assert_eq!(s.chunks[1].kind, ChunkKind::Theirs);
        assert_eq!(s.chunks[1].left_range, [4, 5]);
        assert_eq!(s.conflicts, 0);
    }

    #[test]
    fn colliding_changes_conflict() {
        let s = snap("a\nshared\nc\n", "a\nours\nc\n", "a\ntheirs\nc\n");
        assert_eq!(s.chunks.len(), 1);
        let c = &s.chunks[0];
        assert_eq!(c.kind, ChunkKind::Conflict);
        assert_eq!(s.conflicts, 1);
        assert!(!c.left_emphasis.is_empty());
        assert!(!c.right_emphasis.is_empty());
    }

    #[test]
    fn identical_changes_agree() {
        let s = snap("a\nx\nc\n", "a\ny\nc\n", "a\ny\nc\n");
        assert_eq!(s.chunks.len(), 1);
        assert_eq!(s.chunks[0].kind, ChunkKind::Agree);
        assert_eq!(s.conflicts, 0);
    }

    #[test]
    fn added_and_deleted_visuals() {
        // ours 中间插入一行 → Added, base 侧空区间
        let s = snap("a\nb\n", "a\nnew\nb\n", "a\nb\n");
        assert_eq!(s.chunks[0].visual, ChunkVisual::Added);
        assert_eq!(s.chunks[0].result_range[0], s.chunks[0].result_range[1]);

        // theirs 删除一行 → Deleted, 该侧空区间
        let s = snap("a\nb\nc\n", "a\nb\nc\n", "a\nc\n");
        assert_eq!(s.chunks[0].kind, ChunkKind::Theirs);
        assert_eq!(s.chunks[0].visual, ChunkVisual::Deleted);
        assert_eq!(s.chunks[0].right_range[0], s.chunks[0].right_range[1]);
    }

    #[test]
    fn insertions_at_same_point_conflict() {
        let s = snap("a\nb\n", "a\nX\nb\n", "a\nY\nb\n");
        assert_eq!(s.chunks.len(), 1);
        assert_eq!(s.chunks[0].kind, ChunkKind::Conflict);
    }

    #[test]
    fn oversized_input_degrades_to_single_conflict() {
        let big = "x\n".repeat(1_100_000); // > 2MB
        let s = snap(&big, "a\n", "b\n");
        assert_eq!(s.chunks.len(), 1);
        assert_eq!(s.chunks[0].kind, ChunkKind::Conflict);
    }

    #[test]
    fn overlong_lines_skip_word_emphasis() {
        let base = format!("{}\n", "a".repeat(MAX_EMPHASIS_LINE_CHARS + 1));
        let ours = format!("{}\n", "b".repeat(MAX_EMPHASIS_LINE_CHARS + 1));
        let s = snap(&base, &ours, &base);
        assert_eq!(s.chunks.len(), 1);
        assert_eq!(s.chunks[0].kind, ChunkKind::Ours);
        assert!(s.chunks[0].left_emphasis.is_empty());
    }

    #[test]
    fn utf16_emphasis_offsets_for_cjk() {
        let s = snap("你好世界\n", "你好中国\n", "你好世界\n");
        let c = &s.chunks[0];
        assert_eq!(c.kind, ChunkKind::Ours);
        assert_eq!(c.left_emphasis, vec![[0, 2, 4]]);
    }
}
