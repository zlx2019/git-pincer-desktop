//! Stateless git plumbing: every call shells out to the user's git binary,
//! inheriting their credentials, hooks and rerere configuration.
//! Arguments are always passed as arrays (never through a shell).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Output, Stdio};
use std::sync::mpsc;
use std::thread::JoinHandle;

use serde::{Deserialize, Serialize};

use crate::error::ShellError;

/// 需要清洗的宿主 git 环境变量: 防止嵌套 git 调用(如钩子中启动本程序)劫持仓库定位
const SCRUBBED_ENV: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_NAMESPACE",
    "GIT_PREFIX",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
];

/// 进行中的合并类操作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Op {
    /// git merge
    Merge,
    /// git rebase
    Rebase,
    /// git cherry-pick
    CherryPick,
    /// git revert
    Revert,
}

impl Op {
    /// 对应的 git 子命令名(--continue / --abort 的宿主命令)
    pub fn name(self) -> &'static str {
        match self {
            Op::Merge => "merge",
            Op::Rebase => "rebase",
            Op::CherryPick => "cherry-pick",
            Op::Revert => "revert",
        }
    }
}

/// 冲突文件某一侧的状态, 由缺失的 index stage 推导
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SideStatus {
    /// 双方均存在且与 base 不同
    Modified,
    /// 该侧 stage 缺失(此侧删除了文件)
    Deleted,
    /// base stage 缺失(双方各自新增)
    Added,
}

/// Conflicts 列表页的一行
#[derive(Debug, Clone, Serialize)]
pub struct FileRow {
    /// 相对仓库根的路径
    pub path: String,
    /// 我方(stage 2)状态
    pub yours: SideStatus,
    /// 对方(stage 3)状态
    pub theirs: SideStatus,
    /// 是否二进制(走 pick-one, 不进三栏)
    pub binary: bool,
}

/// 一个冲突文件的三方内容, 缺失侧为空串
#[derive(Debug, Serialize)]
pub struct ThreeWay {
    /// 共同祖先(stage 1)
    pub base: String,
    /// 我方(stage 2)
    pub ours: String,
    /// 对方(stage 3)
    pub theirs: String,
}

/// 整文件取侧
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PickSide {
    /// 取我方(stage 2)
    Yours,
    /// 取对方(stage 3)
    Theirs,
}

/// 可从菜单发起的操作
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LaunchKind {
    /// `git pull`(使用当前分支的跟踪配置)
    Pull,
    /// `git merge <branch>`
    Merge,
    /// `git rebase <branch>`
    Rebase,
    /// `git cherry-pick <commit...>`
    CherryPick,
    /// `git revert <commit...>`
    Revert,
}

impl LaunchKind {
    /// 对应的 git 子命令名
    pub fn name(self) -> &'static str {
        match self {
            LaunchKind::Pull => "pull",
            LaunchKind::Merge => "merge",
            LaunchKind::Rebase => "rebase",
            LaunchKind::CherryPick => "cherry-pick",
            LaunchKind::Revert => "revert",
        }
    }
}

/// 本地分支(merge/rebase 选择用)
#[derive(Debug, Serialize)]
pub struct Branch {
    /// 分支名
    pub name: String,
    /// 是否当前分支
    pub current: bool,
}

/// 供选择的提交(cherry-pick/revert 用)
#[derive(Debug, Serialize)]
pub struct CommitInfo {
    /// 短 sha
    pub sha: String,
    /// 提交标题
    pub subject: String,
    /// 来源分支名(others_only 场景由 %S 溯源; 当前分支自身历史为空串)
    pub branch: String,
}

/// `ls-files -u` 的聚合结果: 冲突路径(首现顺序) + 每路径的 stage 存在性
type StageIndex = (Vec<String>, HashMap<String, [bool; 3]>);

/// 无状态 git 管道: 仅保存仓库定位, 不缓存任何业务数据
#[derive(Debug, Clone)]
pub struct Repo {
    root: PathBuf,
    git_dir: PathBuf,
}

impl Repo {
    /// 从任意目录向上发现 git 仓库
    pub fn discover(dir: &Path) -> Result<Self, ShellError> {
        let out = git_at(dir, &["rev-parse", "--show-toplevel", "--absolute-git-dir"])?;
        if !out.status.success() {
            return Err(ShellError::NotARepo(dir.display().to_string()));
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let mut lines = text.lines();
        let root = lines.next().unwrap_or_default().to_string();
        let git_dir = lines.next().unwrap_or_default().to_string();
        if root.is_empty() || git_dir.is_empty() {
            return Err(ShellError::NotARepo(dir.display().to_string()));
        }
        Ok(Self {
            root: root.into(),
            git_dir: git_dir.into(),
        })
    }

    /// 工作区根路径
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 探测进行中的操作; rebase 标记目录优先(rebase 途中可能同时出现其他标记文件)
    pub fn op(&self) -> Option<Op> {
        let exists = |name: &str| self.git_dir.join(name).exists();
        if exists("rebase-merge") || exists("rebase-apply") {
            Some(Op::Rebase)
        } else if exists("MERGE_HEAD") {
            Some(Op::Merge)
        } else if exists("CHERRY_PICK_HEAD") {
            Some(Op::CherryPick)
        } else if exists("REVERT_HEAD") {
            Some(Op::Revert)
        } else {
            None
        }
    }

    /// 双方展示标签: yours = 当前分支(游离 HEAD 优先解析 rebase 的 onto 分支, 再退化为短 sha),
    /// theirs 按操作类型取对端
    pub fn labels(&self, op: Option<Op>) -> (String, String) {
        let yours = {
            let name = self.out_line(&["rev-parse", "--abbrev-ref", "HEAD"]);
            if name.is_empty() || name == "HEAD" {
                self.rebase_onto_label()
                    .unwrap_or_else(|| self.out_line(&["rev-parse", "--short", "HEAD"]))
            } else {
                name
            }
        };
        let theirs = match op {
            Some(Op::Merge) => self.ref_label("MERGE_HEAD"),
            Some(Op::Rebase) => {
                let name = self.rebase_head_name();
                if name.is_empty() {
                    self.out_line(&["rev-parse", "--short", "REBASE_HEAD"])
                } else {
                    name
                }
            }
            Some(Op::CherryPick) => self.out_line(&["rev-parse", "--short", "CHERRY_PICK_HEAD"]),
            Some(Op::Revert) => self.out_line(&["rev-parse", "--short", "REVERT_HEAD"]),
            None => String::new(),
        };
        (yours, theirs)
    }

    /// 冲突文件列表: `ls-files -u` 按路径聚合, 由缺失 stage 推导两侧状态
    pub fn conflicts(&self) -> Result<Vec<FileRow>, ShellError> {
        let (order, stages) = self.conflict_stages(&[])?;
        Ok(order
            .into_iter()
            .map(|path| {
                let [base, ours, theirs] = stages[&path];
                FileRow {
                    yours: side_status(base, ours),
                    theirs: side_status(base, theirs),
                    binary: self.sniff_binary(&path),
                    path,
                }
            })
            .collect())
    }

    /// `ls-files -u` 输出解析: 冲突路径(首现顺序)与每路径的 stage 存在性;
    /// paths 非空时仅查询这些路径(pathspec 过滤在 git 侧完成)
    fn conflict_stages(&self, paths: &[String]) -> Result<StageIndex, ShellError> {
        let mut args: Vec<&str> = vec!["ls-files", "-u", "-z"];
        if !paths.is_empty() {
            args.push("--");
            args.extend(paths.iter().map(String::as_str));
        }
        let out = self.run_ok(&args)?;
        let text = String::from_utf8_lossy(&out.stdout);
        let mut order: Vec<String> = Vec::new();
        let mut stages: HashMap<String, [bool; 3]> = HashMap::new();
        // 记录格式: "<mode> <oid> <stage>\t<path>", NUL 分隔
        for record in text.split('\0').filter(|r| !r.is_empty()) {
            let Some((meta, path)) = record.split_once('\t') else {
                continue;
            };
            let Some(stage @ 1..=3) = meta
                .rsplit(' ')
                .next()
                .and_then(|s| s.parse::<usize>().ok())
            else {
                continue;
            };
            let entry = stages.entry(path.to_string()).or_insert_with(|| {
                order.push(path.to_string());
                [false; 3]
            });
            entry[stage - 1] = true;
        }
        Ok((order, stages))
    }

    /// 读取三方内容; 缺失 stage 返回空串; UTF-8 lossy 解码(二进制不走此路径)
    pub fn read_three(&self, path: &str) -> ThreeWay {
        let read = |stage: u8| -> String {
            let spec = format!(":{stage}:{path}");
            self.run(&["show", &spec])
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                .unwrap_or_default()
        };
        ThreeWay {
            base: read(1),
            ours: read(2),
            theirs: read(3),
        }
    }

    /// 整文件取一侧: 该侧存在则 checkout+add; 该侧为"删除"则从索引与工作区移除。
    /// stage 查询限定在传入路径(全量 conflicts() 含逐文件二进制嗅探, 批量场景浪费)
    pub fn accept_side(&self, paths: &[String], side: PickSide) -> Result<(), ShellError> {
        let (_, stages) = self.conflict_stages(paths)?;
        for path in paths {
            let Some(&[base, ours, theirs]) = stages.get(path) else {
                continue;
            };
            let (status, flag) = match side {
                PickSide::Yours => (side_status(base, ours), "--ours"),
                PickSide::Theirs => (side_status(base, theirs), "--theirs"),
            };
            if status == SideStatus::Deleted {
                self.run_ok(&["rm", "-f", "--ignore-unmatch", "--", path])?;
            } else {
                self.run_ok(&["checkout", flag, "--", path])?;
                self.run_ok(&["add", "--", path])?;
            }
        }
        Ok(())
    }

    /// 写入解决结果并暂存
    pub fn save_result(&self, path: &str, text: &str) -> Result<(), ShellError> {
        std::fs::write(self.root.join(path), text)?;
        self.run_ok(&["add", "--", path])?;
        Ok(())
    }

    /// 流式执行 git: 输出逐行回调。注入 GIT_EDITOR=true(操作可能自动提交)、
    /// GIT_TERMINAL_PROMPT=0(无终端可用, 让 git 立即失败而不是挂起等待输入)
    pub fn run_streaming(
        &self,
        args: &[&str],
        mut on_line: impl FnMut(&'static str, String),
    ) -> Result<ExitStatus, ShellError> {
        let mut cmd = Command::new("git");
        cmd.current_dir(&self.root)
            .args(args)
            .env("GIT_EDITOR", "true")
            .env("GIT_SEQUENCE_EDITOR", "true")
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for key in SCRUBBED_ENV {
            cmd.env_remove(key);
        }
        let mut child = cmd.spawn()?;
        let (tx, rx) = mpsc::channel::<(&'static str, String)>();
        let mut readers: Vec<JoinHandle<()>> = Vec::new();
        if let Some(out) = child.stdout.take() {
            readers.push(spawn_line_reader("stdout", out, tx.clone()));
        }
        if let Some(err) = child.stderr.take() {
            readers.push(spawn_line_reader("stderr", err, tx.clone()));
        }
        drop(tx);
        // 两个 reader 线程结束后通道关闭, 循环随之退出
        for (stream, line) in rx {
            on_line(stream, line);
        }
        for reader in readers {
            let _ = reader.join();
        }
        Ok(child.wait()?)
    }

    /// 执行 `git <op> --continue`
    pub fn continue_op(
        &self,
        op: Op,
        on_line: impl FnMut(&'static str, String),
    ) -> Result<ExitStatus, ShellError> {
        self.run_streaming(&[op.name(), "--continue"], on_line)
    }

    /// 从菜单发起操作(pull 无目标, merge/rebase 单目标, cherry-pick/revert 可多提交)
    pub fn launch(
        &self,
        kind: LaunchKind,
        targets: &[String],
        on_line: impl FnMut(&'static str, String),
    ) -> Result<ExitStatus, ShellError> {
        let mut args = vec![kind.name()];
        args.extend(targets.iter().map(String::as_str));
        self.run_streaming(&args, on_line)
    }

    /// 中止当前操作(`git <op> --abort`)
    pub fn abort(&self, op: Op) -> Result<(), ShellError> {
        self.run_ok(&[op.name(), "--abort"])?;
        Ok(())
    }

    /// 切换分支(菜单顶栏入口, 仅在无进行中操作时调用); 工作区不允许切换时由 git 拒绝并携带 stderr
    pub fn switch(&self, name: &str) -> Result<(), ShellError> {
        self.run_ok(&["switch", name])?;
        Ok(())
    }

    /// 本地分支列表(带当前分支标记)
    pub fn branches(&self) -> Result<Vec<Branch>, ShellError> {
        let out = self.run_ok(&["branch", "--format=%(HEAD) %(refname:short)"])?;
        Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| {
                let name = line.get(1..)?.trim();
                if name.is_empty() {
                    return None;
                }
                Some(Branch {
                    name: name.to_string(),
                    current: line.starts_with('*'),
                })
            })
            .collect())
    }

    /// 最近提交列表; others_only 时只列当前分支尚未包含的提交(cherry-pick 场景)。
    /// 字段 NUL 分隔防止标题空格干扰; %S 记录提交经由哪个 ref 到达
    pub fn recent_commits(
        &self,
        others_only: bool,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, ShellError> {
        let n = limit.to_string();
        let mut args = vec![
            "log",
            "--no-decorate",
            "--pretty=format:%h%x00%S%x00%s",
            "-n",
            &n,
        ];
        if others_only {
            args.extend(["--all", "--not", "HEAD"]);
        }
        let out = self.run_ok(&args)?;
        Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| {
                let mut parts = line.splitn(3, '\0');
                let sha = parts.next()?.to_string();
                let source = parts.next()?.trim();
                let subject = parts.next()?.to_string();
                let name = source
                    .trim_start_matches("refs/heads/")
                    .trim_start_matches("refs/remotes/")
                    .trim_start_matches("refs/tags/");
                // 当前分支自身的历史(HEAD 遍历)不标注来源
                let branch = if name == "HEAD" { "" } else { name };
                Some(CommitInfo {
                    sha,
                    subject,
                    branch: branch.to_string(),
                })
            })
            .collect())
    }

    /// 未提交变更条目数(`status --porcelain` 的行数, 菜单页提示用)
    pub fn dirty_count(&self) -> Result<usize, ShellError> {
        let out = self.run_ok(&["status", "--porcelain"])?;
        Ok(out
            .stdout
            .split(|b| *b == b'\n')
            .filter(|l| !l.is_empty())
            .count())
    }

    /// 在仓库根执行 git 并捕获输出
    fn run(&self, args: &[&str]) -> Result<Output, ShellError> {
        git_at(&self.root, args)
    }

    /// 执行 git 并要求零退出, 否则携带 stderr 报错
    fn run_ok(&self, args: &[&str]) -> Result<Output, ShellError> {
        let out = self.run(args)?;
        if out.status.success() {
            Ok(out)
        } else {
            Err(ShellError::Git {
                args: args.join(" "),
                stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
            })
        }
    }

    /// 执行 git 取 stdout 首行; 失败返回空串(供标签类"尽力而为"场景)
    fn out_line(&self, args: &[&str]) -> String {
        self.run(args)
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_string()
            })
            .unwrap_or_default()
    }

    /// 把 MERGE_HEAD 之类的引用解析成人类可读标签: 优先分支名, 退化为短 sha
    fn ref_label(&self, refname: &str) -> String {
        let name = self.out_line(&["name-rev", "--name-only", "--exclude=tags/*", refname]);
        let name = name.trim_start_matches("remotes/").to_string();
        // name-rev 解析不出干净分支名(undefined / feature~2 之类)时退回短 sha
        if name.is_empty() || name == "undefined" || name.contains('~') || name.contains('^') {
            self.out_line(&["rev-parse", "--short", refname])
        } else {
            name
        }
    }

    /// rebase 进行中 onto 侧的可读标签: 读标记目录里的 onto sha, 有分支正指向它则用分支名,
    /// 否则退化为该 sha 的短形式; 非 rebase 状态(标记文件缺失)返回 None
    fn rebase_onto_label(&self) -> Option<String> {
        let sha = ["rebase-merge", "rebase-apply"].iter().find_map(|dir| {
            std::fs::read_to_string(self.git_dir.join(dir).join("onto"))
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })?;
        // for-each-ref 只列真实引用, 不含游离 HEAD 的伪条目; 多分支同指一处时取首个
        let branch = self.out_line(&[
            "for-each-ref",
            "refs/heads",
            "--points-at",
            &sha,
            "--format=%(refname:short)",
        ]);
        if !branch.is_empty() {
            return Some(branch);
        }
        let short = self.out_line(&["rev-parse", "--short", &sha]);
        (!short.is_empty()).then_some(short)
    }

    /// rebase 中被变基的分支名(rebase-merge/head-name), 缺失返回空串
    fn rebase_head_name(&self) -> String {
        for dir in ["rebase-merge", "rebase-apply"] {
            if let Ok(s) = std::fs::read_to_string(self.git_dir.join(dir).join("head-name")) {
                return s.trim().trim_start_matches("refs/heads/").to_string();
            }
        }
        String::new()
    }

    /// 工作区文件前 8KB 含 NUL 即视为二进制; 文件缺失按文本处理(走 Accept 流程)
    fn sniff_binary(&self, path: &str) -> bool {
        let Ok(file) = std::fs::File::open(self.root.join(path)) else {
            return false;
        };
        let mut buf = Vec::with_capacity(8192);
        if file.take(8192).read_to_end(&mut buf).is_err() {
            return false;
        }
        buf.contains(&0)
    }
}

/// 由 base 与该侧 stage 的存在性推导单侧状态
fn side_status(base: bool, side: bool) -> SideStatus {
    match (base, side) {
        (_, false) => SideStatus::Deleted,
        (false, true) => SideStatus::Added,
        (true, true) => SideStatus::Modified,
    }
}

/// 以 dir 为工作目录执行 git; 所有 git 调用的统一入口, 集中做环境清洗
fn git_at(dir: &Path, args: &[&str]) -> Result<Output, ShellError> {
    let mut cmd = Command::new("git");
    cmd.current_dir(dir).args(args);
    for key in SCRUBBED_ENV {
        cmd.env_remove(key);
    }
    Ok(cmd.output()?)
}

/// 后台线程逐行读取子进程管道并发往通道
fn spawn_line_reader(
    stream: &'static str,
    pipe: impl Read + Send + 'static,
    tx: mpsc::Sender<(&'static str, String)>,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let reader = BufReader::new(pipe);
        for line in reader.lines().map_while(Result::ok) {
            let _ = tx.send((stream, line));
        }
    })
}
