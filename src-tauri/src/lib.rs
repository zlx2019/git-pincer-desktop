//! Tauri shell entry point: plugin/state/command registration and system-tray wiring.

mod commands;
pub mod error;
pub mod merge;
pub mod repo;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

/// 唤回主窗口(托盘菜单/图标点击与 macOS Dock 重开共用)
fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

/// 建系统托盘: 菜单 = 显示窗口 / 退出。
/// macOS 左键即弹菜单(菜单栏惯例); Windows/Linux 菜单挂右键, 左键单击直接唤回窗口
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("git-pincer")
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(|app, e| match e.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // macOS 左键已被菜单接管不会走到这里; 其余平台左键抬起时唤回
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// 应用入口: 注册插件、全局状态与全部命令; 关窗收进托盘, 退出走托盘菜单
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::set_window_form,
            commands::repo_open,
            commands::conflicts,
            commands::read_three,
            commands::open_merge,
            commands::accept_side,
            commands::save_result,
            commands::continue_op,
            commands::abort_op,
            commands::recent_repos,
            commands::recent_remove,
            commands::launch_op,
            commands::branches,
            commands::switch_branch,
            commands::commits,
        ])
        .setup(|app| {
            build_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // 关窗不退出: 拦截关闭请求隐藏窗口(会话状态原样保留), 真正退出走托盘"退出"或 ⌘Q
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!());
    let app = match app {
        Ok(app) => app,
        Err(e) => {
            // GUI 环境没有可交互终端, 尽力输出后以非零码退出
            eprintln!("fatal: failed to build tauri application: {e}");
            std::process::exit(1);
        }
    };
    app.run(|_app, _event| {
        // macOS: 窗口隐藏后点 Dock 图标重开
        #[cfg(target_os = "macos")]
        if let tauri::RunEvent::Reopen { .. } = _event {
            show_main_window(_app);
        }
    });
}
