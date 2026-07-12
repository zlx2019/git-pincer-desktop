//! Tauri command layer: thin async wrappers around the git plumbing.
//! All blocking git calls run inside `spawn_blocking`.

use std::path::Path;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::error::ShellError;
use crate::merge::MergeSnapshot;
use crate::repo::{Branch, CommitInfo, FileRow, LaunchKind, Op, PickSide, Repo, ThreeWay};
use crate::settings::{
    AppTheme, COMPACT_DEFAULT, COMPACT_MIN, CloseBehavior, LARGE_DEFAULT, LARGE_MIN, Language,
    Settings, WinSize,
};

/// 最近仓库列表的最大长度
const RECENT_LIMIT: usize = 10;

/// 托盘菜单项句柄 (显示窗口, 退出): 语言切换时就地改文案, 不重建托盘
type TrayItems = (
    tauri::menu::MenuItem<tauri::Wry>,
    tauri::menu::MenuItem<tauri::Wry>,
);

/// 全局状态: 当前打开仓库的定位 + 已应用的窗口形态与各形态最近位置(仅本次运行,
/// 不落盘——启动永远居中) + 用户设置(启动时从盘加载) + 托盘/应用菜单句柄(语言切换就地改文案)
#[derive(Default)]
pub struct AppState {
    repo: Mutex<Option<Repo>>,
    win_form: Mutex<Option<WinForm>>,
    win_pos: Mutex<[Option<tauri::PhysicalPosition<i32>>; 2]>,
    settings: Mutex<Settings>,
    tray: Mutex<Option<TrayItems>>,
    menu_settings: Mutex<Option<tauri::menu::MenuItem<tauri::Wry>>>,
}

impl AppState {
    /// 启动时注入从盘上读到的设置(setup 阶段调用一次)
    pub fn init_settings(&self, s: Settings) {
        *self.settings.lock().unwrap_or_else(|e| e.into_inner()) = s;
    }

    /// 当前语言(lib.rs 建托盘时取初始文案)
    pub fn language(&self) -> Language {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .language
    }

    /// 托盘建好后登记菜单项句柄(语言切换要就地改文案)
    pub fn set_tray_items(&self, items: TrayItems) {
        *self.tray.lock().unwrap_or_else(|e| e.into_inner()) = Some(items);
    }

    /// 应用菜单"设置"项句柄登记(仅 macOS 有应用菜单, 其余平台该方法无调用点)
    #[cfg(target_os = "macos")]
    pub fn set_menu_settings_item(&self, item: tauri::menu::MenuItem<tauri::Wry>) {
        *self.menu_settings.lock().unwrap_or_else(|e| e.into_inner()) = Some(item);
    }

    /// 关窗是否收进托盘(lib.rs 的关闭拦截按此分流)
    pub fn close_to_tray(&self) -> bool {
        self.settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .close_behavior
            == CloseBehavior::Tray
    }
}

/// 把设置应用到壳层(setup 与 set_settings 共用): 窗口原生主题与底色
/// (标题栏观感 / resize 露底色) + 托盘菜单文案; 全部尽力而为, 失败不打断
pub fn apply_to_shell(app: &AppHandle, s: &Settings) {
    if let Some(win) = app.get_webview_window("main") {
        let (theme, bg) = match s.theme {
            AppTheme::Dark => (tauri::Theme::Dark, tauri::webview::Color(30, 31, 34, 255)),
            AppTheme::Light => (
                tauri::Theme::Light,
                tauri::webview::Color(255, 255, 255, 255),
            ),
        };
        let _ = win.set_theme(Some(theme));
        let _ = win.set_background_color(Some(bg));
    }
    let state = app.state::<AppState>();
    let tray = state.tray.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((show, quit)) = &*tray {
        let (show_txt, quit_txt) = s.language.tray_labels();
        let _ = show.set_text(show_txt);
        let _ = quit.set_text(quit_txt);
    }
    let menu_settings = state
        .menu_settings
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(item) = &*menu_settings {
        let _ = item.set_text(s.language.settings_label());
    }
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

impl WinForm {
    /// 位置记忆槽位(AppState::win_pos 下标)
    const fn idx(self) -> usize {
        match self {
            Self::Compact => 0,
            Self::Large => 1,
        }
    }
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

/// `git://output` 事件载荷元素: launch/continue 过程的一行输出(事件按批推送)
#[derive(Debug, Clone, Serialize)]
pub struct OutputLine {
    /// 来源流: stdout / stderr
    pub stream: &'static str,
    /// 行内容
    pub line: String,
}

/// 把 repo 层聚好的一批输出行序列化为单个 `git://output` 事件(失败静默, 输出是尽力而为)
fn emit_output(app: &AppHandle, lines: Vec<(&'static str, String)>) {
    let batch: Vec<OutputLine> = lines
        .into_iter()
        .map(|(stream, line)| OutputLine { stream, line })
        .collect();
    let _ = app.emit("git://output", batch);
}

/// 应用窗口形态: 最小尺寸/尺寸/定位一次完成(单次 IPC, 无多段跳变);
/// 形态未变时为空操作——不把用户手动移动或调整过的窗口拽回屏幕中心。
/// 尺寸规则: 用户手动调整过的尺寸按形态记进设置落盘(切换时快照旧形态),
/// 应用时记忆值优先(钳到形态最小尺寸), 没调过用出厂默认——跨启动贴合使用习惯。
/// 定位规则: 切换时记住旧形态当前位置(仅内存), 该形态本次运行内出现过就原位恢复
/// (位置需仍落在某块屏幕上, 防拔外接屏后恢复到屏外), 首次出现(含启动)居中——
/// 冲突处理完/失败回小窗时回到进大窗前的位置
#[tauri::command]
pub async fn set_window_form(
    app: AppHandle,
    state: State<'_, AppState>,
    form: WinForm,
) -> Result<(), ShellError> {
    let prev = {
        let mut cur = state.win_form.lock().unwrap_or_else(|e| e.into_inner());
        if *cur == Some(form) {
            return Ok(());
        }
        cur.replace(form)
    };
    let Some(win) = app.get_webview_window("main") else {
        return Ok(());
    };
    if let Some(prev) = prev {
        if let Ok(pos) = win.outer_position() {
            state.win_pos.lock().unwrap_or_else(|e| e.into_inner())[prev.idx()] = Some(pos);
        }
        remember_form_size(&app, &state, &win, prev);
    }
    let (min, size) = {
        let s = state.settings.lock().unwrap_or_else(|e| e.into_inner());
        match form {
            WinForm::Compact => (
                COMPACT_MIN,
                s.compact_size
                    .map_or(COMPACT_DEFAULT, |z| z.clamp_min(COMPACT_MIN)),
            ),
            WinForm::Large => (
                LARGE_MIN,
                s.large_size
                    .map_or(LARGE_DEFAULT, |z| z.clamp_min(LARGE_MIN)),
            ),
        }
    };
    win.set_min_size(Some(tauri::LogicalSize::new(
        f64::from(min.width),
        f64::from(min.height),
    )))
    .map_err(join_err)?;
    win.set_size(tauri::LogicalSize::new(
        f64::from(size.width),
        f64::from(size.height),
    ))
    .map_err(join_err)?;
    let saved = state.win_pos.lock().unwrap_or_else(|e| e.into_inner())[form.idx()];
    match saved.filter(|p| on_screen(&win, *p)) {
        Some(p) => win.set_position(p).map_err(join_err)?,
        None => win.center().map_err(join_err)?,
    }
    Ok(())
}

/// 把窗口当前逻辑尺寸快照进指定形态的设置字段(纯内存), 返回更新后的设置副本供落盘;
/// 尺寸查询失败返回 None(调用方静默跳过)。用户对窗口的手动调整由此跨启动保留
fn capture_form_size(
    state: &AppState,
    win: &tauri::WebviewWindow,
    form: WinForm,
) -> Option<Settings> {
    let (Ok(size), Ok(scale)) = (win.inner_size(), win.scale_factor()) else {
        return None;
    };
    let logical = size.to_logical::<f64>(scale);
    let ws = WinSize {
        width: logical.width.round() as u32,
        height: logical.height.round() as u32,
    };
    let mut s = state.settings.lock().unwrap_or_else(|e| e.into_inner());
    match form {
        WinForm::Compact => s.compact_size = Some(ws),
        WinForm::Large => s.large_size = Some(ws),
    }
    Some(s.clone())
}

/// 采尺寸并尽力落盘(形态切换路径; 查询/写盘失败静默跳过, 不阻塞形态切换)
fn remember_form_size(
    app: &AppHandle,
    state: &AppState,
    win: &tauri::WebviewWindow,
    form: WinForm,
) {
    if let Some(snapshot) = capture_form_size(state, win, form) {
        persist_settings(app, &snapshot);
    }
}

/// 隐藏/退出前快照当前形态的窗口尺寸(纯内存), 返回待落盘的设置副本——
/// lib.rs 先隐藏窗口再用返回值落盘, 磁盘延迟不垫在"点关闭→窗口消失"的手感里;
/// 形态未知(窗口还没挂过形态)或窗口缺失时返回 None
pub fn remember_win_size(app: &AppHandle) -> Option<Settings> {
    let state = app.state::<AppState>();
    let form = *state.win_form.lock().unwrap_or_else(|e| e.into_inner());
    let (Some(form), Some(win)) = (form, app.get_webview_window("main")) else {
        return None;
    };
    capture_form_size(&state, &win, form)
}

/// 设置快照落盘(尽力而为, 失败静默)
pub fn persist_settings(app: &AppHandle, snapshot: &Settings) {
    if let Some(f) = settings_file(app) {
        let _ = snapshot.save(&f);
    }
}

/// 窗口左上角(标题栏抓手)是否仍落在某块屏幕内; 查询失败按不可见处理(回退居中)
fn on_screen(win: &tauri::WebviewWindow, pos: tauri::PhysicalPosition<i32>) -> bool {
    let Ok(monitors) = win.available_monitors() else {
        return false;
    };
    monitors.iter().any(|m| {
        let (mp, ms) = (m.position(), m.size());
        pos.x >= mp.x
            && pos.x < mp.x + ms.width as i32
            && pos.y >= mp.y
            && pos.y < mp.y + ms.height as i32
    })
}

/// 当前用户设置
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> Result<Settings, ShellError> {
    Ok(state
        .settings
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone())
}

/// 更新设置: 归一化 → 内存 → 壳层应用(窗口主题/托盘文案) → 落盘; 返回归一化结果供前端回同步
#[tauri::command]
pub async fn set_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> Result<Settings, ShellError> {
    let mut s = settings.normalized();
    {
        // 窗口尺寸记忆归 Rust 独占: 前端副本可能陈旧(加载后壳层又快照过),
        // 一律以内存现值为准, 设置对话框的任何改动都不碰这两个字段
        let mut cur = state.settings.lock().unwrap_or_else(|e| e.into_inner());
        s.compact_size = cur.compact_size;
        s.large_size = cur.large_size;
        *cur = s.clone();
    }
    apply_to_shell(&app, &s);
    let file = settings_file(&app)
        .ok_or_else(|| ShellError::Internal("cannot resolve app data dir".into()))?;
    s.save(&file)
        .map_err(|e| ShellError::Internal(format!("failed to write settings: {e}")))?;
    Ok(s)
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
        crate::merge::build_snapshot(&path, three.base, three.ours, three.theirs)
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

/// 驱动 `git <op> --continue`; 输出按批(repo 层聚批)以 `git://output` 事件推送
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
        let status = repo.continue_op(op, |lines| emit_output(&app, lines))?;
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

/// 从菜单发起操作; 输出按批以 `git://output` 事件推送, 结束后按仓库状态分流
#[tauri::command]
pub async fn launch_op(
    app: AppHandle,
    state: State<'_, AppState>,
    kind: LaunchKind,
    targets: Vec<String>,
) -> Result<LaunchOutcome, ShellError> {
    let repo = current(&state)?;
    tauri::async_runtime::spawn_blocking(move || {
        let status = repo.launch(kind, &targets, |lines| emit_output(&app, lines))?;
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

/// 用户设置的存储文件路径(app-data 目录, 应用更新/重装不受影响)
pub fn settings_file(app: &AppHandle) -> Option<std::path::PathBuf> {
    Some(app.path().app_data_dir().ok()?.join("settings.json"))
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
