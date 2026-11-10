//! 云桌面切换器的 Rust 侧。
//!
//! 前端的 noVNC 不能直接拨 VNC 的 TCP 端口，所以连接时由 [`proxy`]
//! 在回环地址上架一座 WebSocket 桥。机器列表和加密后的密码放在本地 JSON。

mod commands;
mod crypto;
mod models;
mod probe;
mod proxy;
mod store;

use std::time::Duration;

use tauri::Manager;
use tauri_plugin_autostart::MacosLauncher;

use crate::commands::{apply_autostart, e2e_flag, probe_targets, AppState};
use crate::store::Store;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .setup(|app| {
            let dir = config_dir(app.handle())?;
            let mut store = Store::open(&dir)?;
            if e2e_flag() {
                let ports =
                    std::env::var("VNC_SWITCHER_E2E_PORTS").unwrap_or_else(|_| "5900,5901".into());
                let parsed = ports
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| value.parse::<u16>())
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|err| format!("E2E 端口无效: {err}"))?;
                let password = std::env::var("VNC_SWITCHER_E2E_PASSWORD").ok();
                store.install_e2e_fixtures(&parsed, password.as_deref())?;
            } else if store.settings().autostart {
                // 配置里开了自启，但桌面项可能被删过，启动时补一次。失败不阻止窗口打开。
                if let Err(err) = apply_autostart(app.handle(), true) {
                    log::warn!("{err}");
                }
            }
            app.manage(AppState {
                store: std::sync::Mutex::new(store),
                proxies: proxy::ProxyManager::new(),
            });
            if e2e_flag() {
                // 验证时把窗口放到桌面左上角，方便截图和发送快捷键。
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.set_position(tauri::PhysicalPosition::new(40, 30));
                    let _ = window.set_size(tauri::PhysicalSize::new(1440, 900));
                    let _ = window.set_always_on_top(true);
                    let _ = window.set_focus();
                }
            }
            spawn_probe_loop(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_machines,
            commands::upsert_machine,
            commands::delete_machine,
            commands::get_settings,
            commands::save_settings,
            commands::config_path,
            commands::connect_machine,
            commands::disconnect_machine,
            commands::session_tcp_count,
            commands::probe_machines,
            commands::e2e_enabled,
            commands::report_e2e,
        ])
        .run(tauri::generate_context!())
        .expect("启动云桌面切换器失败");
}

fn config_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    if let Ok(dir) = std::env::var("VNC_SWITCHER_CONFIG_DIR") {
        let path = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&path)?;
        return Ok(path);
    }
    let path = app.path().app_config_dir()?;
    std::fs::create_dir_all(&path)?;
    Ok(path)
}

fn spawn_probe_loop(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(4)).await;
            let targets = {
                let state = app.state::<AppState>();
                let guard = match state.store.lock() {
                    Ok(store) => store,
                    Err(_) => continue,
                };
                guard.probe_targets()
            };
            if let Ok(events) = probe_targets(targets).await {
                for event in events {
                    commands::emit_probe(&app, &event);
                }
            }
        }
    });
}
