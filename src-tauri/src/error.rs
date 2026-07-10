//! Shell error type shared by all Tauri commands.

use serde::{Serialize, Serializer};

/// 壳层统一错误: 以消息字符串序列化, 由前端 toast 呈现
#[derive(Debug, thiserror::Error)]
pub enum ShellError {
    /// 路径不是 git 仓库
    #[error("Not a git repository: {0}")]
    NotARepo(String),
    /// 尚未打开任何仓库
    #[error("No repository is open")]
    NoRepoOpen,
    /// git 命令非零退出
    #[error("git {args}: {stderr}")]
    Git {
        /// 失败的参数串(空格拼接, 仅用于报错展示)
        args: String,
        /// 子进程 stderr(已裁剪)
        stderr: String,
    },
    /// IO 失败
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// 异步任务 join 失败等内部错误
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Serialize for ShellError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
