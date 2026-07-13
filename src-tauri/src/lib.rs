//! Tauri shell entry point: plugin/state/command registration and system-tray wiring.

mod commands;
pub mod error;
pub mod merge;
pub mod repo;
pub mod settings;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

/// 唤回主窗口(托盘菜单/图标点击与 macOS Dock 重开共用)
fn show_main_window(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

/// 建系统托盘: 菜单 = 显示窗口 / 设置 / 退出(文案随语言设置, 切换时就地更新)。
/// 设置项是 Windows/Linux 的主入口(无应用菜单, 快捷键 Ctrl+, 不可发现);
/// macOS 左键即弹菜单(菜单栏惯例); Windows/Linux 菜单挂右键, 左键单击直接唤回窗口
fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::PredefinedMenuItem;

    let lang = app.state::<commands::AppState>().language();
    let (show_txt, quit_txt) = lang.tray_labels();
    let show = MenuItem::with_id(app, "show", show_txt, true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", lang.settings_label(), true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", quit_txt, true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Pincer")
        .menu(&menu)
        .show_menu_on_left_click(cfg!(target_os = "macos"))
        .on_menu_event(|app, e| match e.id.as_ref() {
            "show" => show_main_window(app),
            "quit" => app.exit(0),
            // "settings" 走 app 级 on_menu_event(与 macOS 应用菜单同 id 共用一份处理)
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
    // macOS 菜单栏惯例: 单色 template 剪影(源 assets/tray.svg), 系统随亮暗菜单栏
    // 与选中态自动反色; 其余平台的托盘保留彩色应用图标
    #[cfg(target_os = "macos")]
    {
        let tpl = tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?;
        tray = tray.icon(tpl).icon_as_template(true);
    }
    #[cfg(not(target_os = "macos"))]
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    app.state::<commands::AppState>()
        .set_tray_items((show, settings, quit));
    Ok(())
}

/// macOS 应用菜单: 在默认菜单的应用子菜单里插入 "设置…"(⌘,, macOS 惯例位 = About 之后)。
/// 仅 macOS——其余平台设应用菜单会给窗口顶部凭空加一条菜单栏, 不符合窗口形态
#[cfg(target_os = "macos")]
fn build_app_menu(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{MenuItemKind, PredefinedMenuItem};

    let label = app
        .state::<commands::AppState>()
        .language()
        .settings_label();
    let menu = Menu::default(app.handle())?;
    let settings = MenuItem::with_id(app, "settings", label, true, Some("Cmd+,"))?;
    if let Some(MenuItemKind::Submenu(app_menu)) = menu.items()?.into_iter().next() {
        // 默认应用子菜单结构: [About, 分隔线, Services, ...] → 设置项插在首个分隔线之后
        let pos = usize::min(2, app_menu.items()?.len());
        app_menu.insert(&settings, pos)?;
        app_menu.insert(&PredefinedMenuItem::separator(app)?, pos + 1)?;
    }
    app.set_menu(menu)?;
    app.state::<commands::AppState>()
        .set_menu_settings_item(settings);
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
            commands::get_settings,
            commands::set_settings,
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
            // 设置先于一切 UI 逻辑加载(关窗行为等在事件回调里同步读取);
            // 过一遍归一化, 手改配置文件的越界值(字号/窗口尺寸)在源头钳掉
            let loaded = commands::settings_file(app.handle())
                .map(|f| settings::Settings::load(&f))
                .unwrap_or_default()
                .normalized();
            app.state::<commands::AppState>()
                .init_settings(loaded.clone());
            build_tray(app)?;
            #[cfg(target_os = "macos")]
            build_app_menu(app)?;
            // 窗口原生主题/底色按持久化设置就位(窗口尚未 show, 无闪变)
            commands::apply_to_shell(app.handle(), &loaded);
            // 记忆的小窗尺寸同样在 show 前就位(启动路由必是小窗形态), 保持启动居中
            if let (Some(ws), Some(win)) = (loaded.compact_size, app.get_webview_window("main")) {
                let _ = win.set_size(tauri::LogicalSize::new(
                    f64::from(ws.width),
                    f64::from(ws.height),
                ));
                let _ = win.center();
            }
            Ok(())
        })
        .on_menu_event(|app, e| {
            // "设置…"(macOS 应用菜单与托盘菜单同 id): 唤回窗口(可能正驻留托盘)
            // 并通知前端弹设置对话框; 托盘的 show/quit 由托盘自己的 on_menu_event 处理
            if e.id.as_ref() == "settings" {
                show_main_window(app);
                let _ = app.emit("app://open-settings", ());
            }
        })
        .on_window_event(|window, event| {
            // 关窗行为由设置决定: 收进托盘(默认, 会话状态原样保留)或直接退出。
            // 尺寸记忆的兜底采集点: 采集(纯内存)在隐藏前, 落盘挪到隐藏后——
            // 磁盘延迟不垫在"点关闭→窗口消失"的手感里
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let app = window.app_handle();
                let snapshot = commands::remember_win_size(app);
                let state = app.state::<commands::AppState>();
                if state.close_to_tray() {
                    api.prevent_close();
                    let _ = window.hide();
                }
                if let Some(s) = snapshot {
                    commands::persist_settings(app, &s);
                }
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
    app.run(|app, event| {
        match event {
            // 退出请求(⌘Q/应用菜单/托盘退出都经此): 退出前快照当前形态窗口尺寸并落盘
            tauri::RunEvent::ExitRequested { .. } => {
                if let Some(s) = commands::remember_win_size(app) {
                    commands::persist_settings(app, &s);
                }
            }
            // macOS: 窗口隐藏后点 Dock 图标重开
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => show_main_window(app),
            _ => {}
        }
    });
}
