//! Integration tests for the git plumbing layer, built on real temporary
//! repositories with repo-local config to isolate from the user's settings.
#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use git_pincer_desktop_lib::repo::{LaunchKind, Op, PickSide, Repo, SideStatus};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// 自清理的临时仓库(避免引入 tempfile 依赖)
struct TempRepo {
    dir: PathBuf,
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// 在目录内执行 git 并断言成功; 屏蔽全局/系统配置保证确定性
fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

/// 同 git(), 但允许失败(制造冲突的 merge 预期非零退出)
fn git_may_fail(dir: &Path, args: &[&str]) {
    let _ = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .args(args)
        .output()
        .unwrap();
}

/// 写文件 + add + commit
fn commit_file(dir: &Path, path: &str, content: &[u8], msg: &str) {
    std::fs::write(dir.join(path), content).unwrap();
    git(dir, &["add", "--", path]);
    git(dir, &["commit", "-m", msg]);
}

/// 构造冲突前置现场(main 与 feature 各自演进, 尚未 merge):
/// - conflict.txt: 双方修改(modify/modify)
/// - del.txt:      main 删除 / feature 修改(delete/modify)
/// - bin.dat:      双方修改的二进制
fn conflict_setup() -> TempRepo {
    let dir = std::env::temp_dir().join(format!(
        "git-pincer-desktop-test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-b", "main"]);
    // 仓库本地配置压制可能来自用户环境的干扰项
    git(&dir, &["config", "user.name", "test"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    git(&dir, &["config", "commit.gpgsign", "false"]);
    git(&dir, &["config", "rerere.enabled", "false"]);

    commit_file(
        &dir,
        "conflict.txt",
        b"line1\nshared\nline3\n",
        "base: conflict.txt",
    );
    commit_file(&dir, "del.txt", b"keep me\n", "base: del.txt");
    commit_file(&dir, "bin.dat", b"BIN\x00v0\x00", "base: bin.dat");

    git(&dir, &["switch", "-c", "feature"]);
    commit_file(
        &dir,
        "conflict.txt",
        b"line1\nfeature-change\nline3\n",
        "feature: conflict.txt",
    );
    commit_file(&dir, "del.txt", b"feature edited\n", "feature: del.txt");
    commit_file(&dir, "bin.dat", b"BIN\x00feature\x00", "feature: bin.dat");

    git(&dir, &["switch", "main"]);
    commit_file(
        &dir,
        "conflict.txt",
        b"line1\nmain-change\nline3\n",
        "main: conflict.txt",
    );
    git(&dir, &["rm", "--", "del.txt"]);
    git(&dir, &["commit", "-m", "main: delete del.txt"]);
    commit_file(&dir, "bin.dat", b"BIN\x00main\x00", "main: bin.dat");

    TempRepo { dir }
}

/// 前置现场 + 已发起 merge(处于冲突状态)
fn merge_conflict_repo() -> TempRepo {
    let tmp = conflict_setup();
    git_may_fail(&tmp.dir, &["merge", "feature"]);
    tmp
}

/// 便捷断言: 按路径取行
fn row<'a>(
    rows: &'a [git_pincer_desktop_lib::repo::FileRow],
    path: &str,
) -> &'a git_pincer_desktop_lib::repo::FileRow {
    rows.iter().find(|r| r.path == path).unwrap()
}

#[test]
fn discovers_op_and_labels() {
    let tmp = merge_conflict_repo();
    let repo = Repo::discover(&tmp.dir).unwrap();
    assert_eq!(repo.op(), Some(Op::Merge));
    let (yours, theirs) = repo.labels(repo.op());
    assert_eq!(yours, "main");
    assert_eq!(theirs, "feature");
}

#[test]
fn discover_rejects_non_repo() {
    let dir = std::env::temp_dir().join(format!(
        "git-pincer-desktop-nonrepo-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    // 防止 temp 目录的祖先恰好是仓库: 断言只要求不 panic 且结果合理
    let result = Repo::discover(&dir);
    if let Ok(repo) = result {
        assert!(!repo.root().as_os_str().is_empty());
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn lists_conflicts_with_statuses() {
    let tmp = merge_conflict_repo();
    let repo = Repo::discover(&tmp.dir).unwrap();
    let rows = repo.conflicts().unwrap();
    assert_eq!(rows.len(), 3);

    let c = row(&rows, "conflict.txt");
    assert_eq!(c.yours, SideStatus::Modified);
    assert_eq!(c.theirs, SideStatus::Modified);
    assert!(!c.binary);

    let d = row(&rows, "del.txt");
    assert_eq!(d.yours, SideStatus::Deleted);
    assert_eq!(d.theirs, SideStatus::Modified);

    let b = row(&rows, "bin.dat");
    assert_eq!(b.yours, SideStatus::Modified);
    assert_eq!(b.theirs, SideStatus::Modified);
    assert!(b.binary);
}

#[test]
fn reads_three_way_content() {
    let tmp = merge_conflict_repo();
    let repo = Repo::discover(&tmp.dir).unwrap();
    let three = repo.read_three("conflict.txt");
    assert_eq!(three.base, "line1\nshared\nline3\n");
    assert_eq!(three.ours, "line1\nmain-change\nline3\n");
    assert_eq!(three.theirs, "line1\nfeature-change\nline3\n");

    // 删除侧内容为空串
    let three = repo.read_three("del.txt");
    assert_eq!(three.ours, "");
    assert_eq!(three.theirs, "feature edited\n");
}

#[test]
fn accept_side_resolves_files() {
    let tmp = merge_conflict_repo();
    let repo = Repo::discover(&tmp.dir).unwrap();

    // 文本: 取我方 → 工作区回到 main 版本
    repo.accept_side(&["conflict.txt".into()], PickSide::Yours)
        .unwrap();
    let content = std::fs::read_to_string(tmp.dir.join("conflict.txt")).unwrap();
    assert_eq!(content, "line1\nmain-change\nline3\n");

    // 删除冲突: 取我方(删除侧) → 文件从索引与工作区消失
    repo.accept_side(&["del.txt".into()], PickSide::Yours)
        .unwrap();
    assert!(!tmp.dir.join("del.txt").exists());

    // 二进制: 取对方 → 工作区为 feature 字节
    repo.accept_side(&["bin.dat".into()], PickSide::Theirs)
        .unwrap();
    assert_eq!(
        std::fs::read(tmp.dir.join("bin.dat")).unwrap(),
        b"BIN\x00feature\x00"
    );

    assert!(repo.conflicts().unwrap().is_empty());
}

#[test]
fn save_result_stages_content() {
    let tmp = merge_conflict_repo();
    let repo = Repo::discover(&tmp.dir).unwrap();
    repo.save_result("conflict.txt", "line1\nmanually merged\nline3\n")
        .unwrap();
    let rows = repo.conflicts().unwrap();
    assert!(rows.iter().all(|r| r.path != "conflict.txt"));
    let content = std::fs::read_to_string(tmp.dir.join("conflict.txt")).unwrap();
    assert_eq!(content, "line1\nmanually merged\nline3\n");
}

#[test]
fn continue_finishes_merge_after_resolution() {
    let tmp = merge_conflict_repo();
    let repo = Repo::discover(&tmp.dir).unwrap();
    repo.accept_side(
        &["conflict.txt".into(), "del.txt".into(), "bin.dat".into()],
        PickSide::Theirs,
    )
    .unwrap();

    let mut lines = Vec::new();
    let status = repo
        .continue_op(Op::Merge, |stream, line| {
            lines.push(format!("{stream}: {line}"))
        })
        .unwrap();
    assert!(status.success(), "continue failed: {lines:?}");
    assert_eq!(repo.op(), None);
    assert!(repo.conflicts().unwrap().is_empty());
}

#[test]
fn launch_merge_reports_conflicts() {
    let tmp = conflict_setup();
    let repo = Repo::discover(&tmp.dir).unwrap();
    assert_eq!(repo.op(), None);

    let mut lines = Vec::new();
    let status = repo
        .launch(LaunchKind::Merge, &["feature".into()], |_, line| {
            lines.push(line)
        })
        .unwrap();
    assert!(!status.success());
    assert_eq!(repo.op(), Some(Op::Merge));
    assert_eq!(repo.conflicts().unwrap().len(), 3);
    assert!(
        lines.iter().any(|l| l.to_lowercase().contains("conflict")),
        "expected conflict output, got: {lines:?}"
    );
}

#[test]
fn lists_branches_and_commits() {
    let tmp = conflict_setup();
    let repo = Repo::discover(&tmp.dir).unwrap();

    let branches = repo.branches().unwrap();
    assert!(branches.iter().any(|b| b.name == "main" && b.current));
    assert!(branches.iter().any(|b| b.name == "feature" && !b.current));

    // cherry-pick 场景: feature 独有提交
    let others = repo.recent_commits(true, 20).unwrap();
    assert!(
        others
            .iter()
            .any(|c| c.subject.contains("feature: conflict.txt"))
    );
    // revert 场景: 当前分支提交
    let mine = repo.recent_commits(false, 20).unwrap();
    assert!(
        mine.iter()
            .any(|c| c.subject.contains("main: conflict.txt"))
    );

    assert_eq!(repo.dirty_count().unwrap(), 0);
}

#[test]
fn abort_restores_clean_state() {
    let tmp = merge_conflict_repo();
    let repo = Repo::discover(&tmp.dir).unwrap();
    repo.abort(Op::Merge).unwrap();
    assert_eq!(repo.op(), None);
    assert!(repo.conflicts().unwrap().is_empty());
}
