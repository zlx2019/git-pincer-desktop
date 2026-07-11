//! Tauri command layer: thin async wrappers around the git plumbing.
//! All blocking git calls run inside `spawn_blocking`.

use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::ShellError;
use crate::merge::MergeSnapshot;
use crate::repo::{Branch, CommitInfo, FileRow, LaunchKind, Op, PickSide, Repo, ThreeWay};

/// 最近仓库列表的最大长度
const RECENT_LIMIT: usize = 10;

/// 全局状态: 当前打开仓库的定位 + 已应用的窗口形态
#[derive(Default)]
pub struct AppState {
    repo: Mutex<Option<Repo>>,
    win_form: Mutex<Option<WinForm>>,
}

/// 窗口形态(路由决定): 小窗 = 打开页/菜单, 大窗 = 冲突列表/三栏
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WinForm {
    /// 紧凑小窗 420×640
    Compact,
    /// 大窗 1280×800
    Large,
}

/// 仓库概要: 打开页与列表页头部所需的全部信息
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoInfo {
    /// 工作区根路径
    pub root: String,
    /// 进行中的操作(无则 null)
    pub op: Option<Op>,
    /// 我方标签(分支名或短 sha)
    pub yours_label: String,
    /// 对方标签
    pub theirs_label: String,
    /// 未提交变更条目数(菜单页提示用)
    pub dirty: usize,
}

/// 发起操作的结果
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum LaunchOutcome {
    /// 顺利完成, 无冲突
    CleanDone,
    /// 出现冲突, 进入接管流程
    Conflicts {
        /// 冲突文件
        files: Vec<FileRow>,
    },
    /// 失败且无冲突(脏工作区 / 网络 / 无上游等, 详情已在输出流)
    Failed {
        /// 失败摘要
        message: String,
    },
}

/// continue 一轮的结果
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RoundOutcome {
    /// 操作全部完成, 仓库回到干净状态
    Done,
    /// 下一轮又出现冲突(多轮 rebase / 多 commit cherry-pick)
    NextRound {
        /// 新一轮的冲突文件
        files: Vec<FileRow>,
    },
    /// continue 失败且并非新冲突(如钩子拒绝)
    Failed {
        /// 失败摘要(详细输出已经由事件流推送)
        message: String,
    },
}

/// `git://output` 事件载荷: continue 过程的一行输出
#[derive(Debug, Clone, Serialize)]
pub struct OutputLine {
    /// 来源流: stdout / stderr
    pub stream: &'static str,
    /// 行内容
    pub line: String,
}

/// 应用窗口形态: 最小尺寸/尺寸/居中一次完成(单次 IPC, 无多段跳变);
/// 形态未变时为空操作——不把用户手动移动或调整过的窗口拽回屏幕中心
#[tauri::command]
pub async fn set_window_form(
    app: AppHandle,
    state: State<'_, AppState>,
    form: WinForm,
) -> Result<(), ShellError> {
    {
        let mut cur = state.win_form.lock().unwrap_or_else(|e| e.into_inner());
        if *cur == Some(form) {
            return Ok(());
        }
        *cur = Some(form);
    }
    let Some(win) = app.get_webview_window("main") else {
        return Ok(());
    };
    let ((min_w, min_h), (w, h)) = match form {
        WinForm::Compact => ((380.0, 520.0), (420.0, 640.0)),
        WinForm::Large => ((960.0, 640.0), (1280.0, 800.0)),
    };
    win.set_min_size(Some(tauri::LogicalSize::new(min_w, min_h)))
        .map_err(join_err)?;
    win.set_size(tauri::LogicalSize::new(w, h))
        .map_err(join_err)?;
    win.center().map_err(join_err)?;
    Ok(())
}

/// 打开新仓库(path 有值)或重探当前仓库(path 为空), 返回概要
#[tauri::command]
pub async fn repo_open(
    app: AppHandle,
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<RepoInfo, ShellError> {
    let repo = match path {
        Some(p) => tauri::async_runtime::spawn_blocking(move || Repo::discover(Path::new(&p)))
            .await
            .map_err(join_err)??,
        None => current(&state)?,
    };
    let probe = repo.clone();
    let info = tauri::async_runtime::spawn_blocking(move || {
        let op = probe.op();
        let (yours_label, theirs_label) = probe.labels(op);
        RepoInfo {
            root: probe.root().display().to_string(),
            op,
            yours_label,
            theirs_label,
            dirty: probe.dirty_count().unwrap_or(0),
        }
    })
    .await
    .map_err(join_err)?;
    *lock(&state) = Some(repo);
    push_recent(&app, &info.root);
    Ok(info)
}

/// 当前冲突文件列表
#[tauri::command]
pub async fn conflicts(state: State<'_, AppState>) -> Result<Vec<FileRow>, ShellError> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || repo.conflicts())
        .await
        .map_err(join_err)?
}

/// 读取单个文件的三方内容(三栏 / 二进制预览共用)
#[tauri::command]
pub async fn read_three(state: State<'_, AppState>, path: String) -> Result<ThreeWay, ShellError> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || repo.read_three(&path))
        .await
        .map_err(join_err)
}

/// 整文件取侧(列表页 Accept Yours/Theirs 与二进制 pick-one 共用)
#[tauri::command]
pub async fn accept_side(
    state: State<'_, AppState>,
    paths: Vec<String>,
    side: PickSide,
) -> Result<(), ShellError> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || repo.accept_side(&paths, side))
        .await
        .map_err(join_err)?
}

/// 构建三栏合并快照(读三方 stage 内容 + 分块引擎)
#[tauri::command]
pub async fn open_merge(
    state: State<'_, AppState>,
    path: String,
) -> Result<MergeSnapshot, ShellError> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        let three = repo.read_three(&path);
        crate::merge::build_snapshot(&path, &three.base, &three.ours, &three.theirs)
    })
    .await
    .map_err(join_err)
}

/// 写入三栏解决结果并暂存
#[tauri::command]
pub async fn save_result(
    state: State<'_, AppState>,
    path: String,
    text: String,
) -> Result<(), ShellError> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || repo.save_result(&path, &text))
        .await
        .map_err(join_err)?
}

/// 驱动 `git <op> --continue`; 输出逐行以 `git://output` 事件推送
#[tauri::command]
pub async fn continue_op(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<RoundOutcome, ShellError> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(op) = repo.op() else {
            return Ok(RoundOutcome::Done);
        };
        let status = repo.continue_op(op, |stream, line| {
            let _ = app.emit("git://output", OutputLine { stream, line });
        })?;
        if status.success() {
            return Ok(RoundOutcome::Done);
        }
        // 非零退出: 有新冲突则是下一轮, 否则如实报失败
        let files = repo.conflicts()?;
        if files.is_empty() {
            Ok(RoundOutcome::Failed {
                message: format!("git {} --continue failed{}", op.name(), exit_suffix(status)),
            })
        } else {
            Ok(RoundOutcome::NextRound { files })
        }
    })
    .await
    .map_err(join_err)?
}

/// 从菜单发起操作; 输出以 `git://output` 事件推送, 结束后按仓库状态分流
#[tauri::command]
pub async fn launch_op(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: LaunchKind,
    targets: Vec<String>,
) -> Result<LaunchOutcome, ShellError> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        let status = repo.launch(kind, &targets, |stream, line| {
            let _ = app.emit("git://output", OutputLine { stream, line });
        })?;
        // 只要出现冲突就接管, 与退出码无关; 干净且零退出才算顺利完成
        let files = repo.conflicts()?;
        if !files.is_empty() {
            return Ok(LaunchOutcome::Conflicts { files });
        }
        if status.success() {
            Ok(LaunchOutcome::CleanDone)
        } else {
            Ok(LaunchOutcome::Failed {
                message: format!("git {} failed{}", kind.name(), exit_suffix(status)),
            })
        }
    })
    .await
    .map_err(join_err)?
}

/// 本地分支列表(merge/rebase 对话框)
#[tauri::command]
pub async fn branches(state: State<'_, AppState>) -> Result<Vec<Branch>, ShellError> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || repo.branches())
        .await
        .map_err(join_err)?
}

/// 切换分支(菜单顶栏分支 chip)
#[tauri::command]
pub async fn switch_branch(state: State<'_, AppState>, name: String) -> Result<(), ShellError> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || repo.switch(&name))
        .await
        .map_err(join_err)?
}

/// 最近提交列表(cherry-pick/revert 对话框)
#[tauri::command]
pub async fn commits(
    state: State<'_, AppState>,
    others_only: bool,
    limit: usize,
) -> Result<Vec<CommitInfo>, ShellError> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || repo.recent_commits(others_only, limit))
        .await
        .map_err(join_err)?
}

/// 中止当前操作
#[tauri::command]
pub async fn abort_op(state: State<'_, AppState>) -> Result<(), ShellError> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        let Some(op) = repo.op() else { return Ok(()) };
        repo.abort(op)
    })
    .await
    .map_err(join_err)?
}

/// 最近打开过的仓库路径(最新在前, 已不存在的目录自动剔除)
#[tauri::command]
pub async fn recent_repos(app: AppHandle) -> Result<Vec<String>, ShellError> {
    Ok(load_recent(&app)
        .into_iter()
        .filter(|p| Path::new(p).is_dir())
        .collect())
}

/// 从最近列表移除一个路径(打开页的删除按钮), 返回更新后的列表
#[tauri::command]
pub async fn recent_remove(app: AppHandle, path: String) -> Result<Vec<String>, ShellError> {
    let mut list = load_recent(&app);
    list.retain(|p| p != &path);
    save_recent(&app, &list);
    Ok(list.into_iter().filter(|p| Path::new(p).is_dir()).collect())
}

/// 取当前仓库的克隆(仅两个 PathBuf), 避免跨 await 持锁
fn current(state: &State<'_, AppState>) -> Result<Repo, ShellError> {
    lock(state).clone().ok_or(ShellError::NoRepoOpen)
}

/// 取状态锁; 锁中毒时继续使用内部值(状态只是仓库定位, 不会处于半更新)
fn lock<'a>(state: &'a State<'_, AppState>) -> std::sync::MutexGuard<'a, Option<Repo>> {
    state.repo.lock().unwrap_or_else(|e| e.into_inner())
}

/// spawn_blocking 的 JoinError 转壳层错误
fn join_err(e: tauri::Error) -> ShellError {
    ShellError::Internal(e.to_string())
}

/// 退出状态的人类可读后缀, 如 " (exit code 1)"
fn exit_suffix(status: std::process::ExitStatus) -> String {
    match status.code() {
        Some(code) => format!(" (exit code {code})"),
        None => " (terminated by signal)".to_string(),
    }
}

/// 最近仓库列表的存储文件路径(app-data 目录)
fn recent_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    let dir = app.path().app_data_dir().ok()?;
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("recent.json"))
}

/// 读取最近仓库列表; 任何失败都按空列表处理(尽力而为)
fn load_recent(app: &AppHandle) -> Vec<String> {
    recent_file(app)
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

/// 持久化最近仓库列表, 失败静默(尽力而为)
fn save_recent(app: &AppHandle, list: &[String]) {
    if let Some(file) = recent_file(app)
        && let Ok(json) = serde_json::to_string_pretty(list)
    {
        let _ = std::fs::write(file, json);
    }
}

/// 把仓库路径插入最近列表头部(去重、截断), 失败静默
fn push_recent(app: &AppHandle, root: &str) {
    let mut list = load_recent(app);
    list.retain(|p| p != root);
    list.insert(0, root.to_string());
    list.truncate(RECENT_LIMIT);
    save_recent(app, &list);
}
