//! Tauri shell entry point: plugin/state/command registration only.

mod commands;
pub mod error;
pub mod merge;
pub mod repo;

/// 应用入口: 注册插件、全局状态与全部命令
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::repo_open,
            commands::conflicts,
            commands::read_three,
            commands::open_merge,
            commands::accept_side,
            commands::save_result,
            commands::continue_op,
            commands::abort_op,
            commands::recent_repos,
            commands::launch_op,
            commands::branches,
            commands::commits,
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        // GUI 环境没有可交互终端, 尽力输出后以非零码退出
        eprintln!("fatal: failed to run tauri application: {e}");
        std::process::exit(1);
    }
}
