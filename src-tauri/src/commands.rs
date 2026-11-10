//! Tauri 命令。业务规则在 `store` / `proxy` / `probe`，这里只做编排。

use std::time::Duration;

use tauri::{AppHandle, Emitter, State};

use crate::models::{ConnectResponse, MachineInput, MachinePublic, ProbeEvent, Settings};
use crate::proxy::ProxyManager;
use crate::store::Store;

pub struct AppState {
    pub store: std::sync::Mutex<Store>,
    pub proxies: ProxyManager,
}

fn lock_store(state: &AppState) -> Result<std::sync::MutexGuard<'_, Store>, String> {
    state.store.lock().map_err(|_| "配置锁已损坏".to_string())
}

#[tauri::command]
pub fn list_machines(state: State<AppState>) -> Result<Vec<MachinePublic>, String> {
    Ok(lock_store(&state)?.machines())
}

#[tauri::command]
pub fn upsert_machine(
    state: State<AppState>,
    input: MachineInput,
) -> Result<MachinePublic, String> {
    lock_store(&state)?.upsert(input)
}

#[tauri::command]
pub fn delete_machine(state: State<AppState>, id: String) -> Result<(), String> {
    state.proxies.stop(&id);
    lock_store(&state)?.delete(&id)
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    Ok(lock_store(&state)?.settings())
}

#[tauri::command]
pub fn config_path(state: State<AppState>) -> Result<String, String> {
    Ok(lock_store(&state)?.config_path().display().to_string())
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<AppState>,
    settings: Settings,
) -> Result<Settings, String> {
    let autostart = settings.autostart;
    let saved = lock_store(&state)?.save_settings(settings)?;
    apply_autostart(&app, autostart)?;
    Ok(saved)
}

#[tauri::command]
pub async fn connect_machine(
    state: State<'_, AppState>,
    id: String,
) -> Result<ConnectResponse, String> {
    let (transport, host, port, direct_url) = lock_store(&state)?.endpoint(&id)?;
    let mut response = lock_store(&state)?.connect_plan(&id)?;
    match transport.as_str() {
        "vnc-tcp" => {
            let info = state.proxies.ensure(&id, &host, port).await?;
            response.ws_url = Some(info.ws_url);
        }
        "websocket" => {
            response.ws_url = direct_url;
            if response.ws_url.is_none() {
                return Err("缺少 WebSocket 直连地址".into());
            }
        }
        "web" => {
            if response.web_url.is_none() {
                return Err("缺少网页地址".into());
            }
        }
        other => return Err(format!("不支持的传输方式 {other}")),
    }
    Ok(response)
}

#[tauri::command]
pub fn disconnect_machine(state: State<AppState>, id: String) -> Result<(), String> {
    state.proxies.stop(&id);
    Ok(())
}

#[tauri::command]
pub fn session_tcp_count(state: State<AppState>, id: String) -> Result<u64, String> {
    Ok(state.proxies.tcp_count(&id))
}

#[tauri::command]
pub async fn probe_machines(state: State<'_, AppState>) -> Result<Vec<ProbeEvent>, String> {
    let targets = lock_store(&state)?.probe_targets();
    probe_targets(targets).await
}

#[tauri::command]
pub fn e2e_enabled() -> bool {
    e2e_flag()
}

#[tauri::command]
pub fn report_e2e(state: State<AppState>, payload: serde_json::Value) -> Result<(), String> {
    if !e2e_flag() {
        return Err("当前不是验证模式".into());
    }
    let mut payload = payload;
    if let Some(object) = payload.as_object_mut() {
        let counts = state.proxies.tcp_counts();
        object.insert(
            "proxyTcp".to_string(),
            serde_json::to_value(counts).unwrap_or(serde_json::Value::Null),
        );
    }
    let path = std::env::var("VNC_SWITCHER_E2E_REPORT")
        .unwrap_or_else(|_| "/tmp/vnc-switcher-e2e.json".into());
    let body = serde_json::to_vec_pretty(&payload).map_err(|err| err.to_string())?;
    std::fs::write(path, body).map_err(|err| format!("写入验证报告失败: {err}"))?;
    Ok(())
}

pub fn e2e_flag() -> bool {
    std::env::var("VNC_SWITCHER_E2E").ok().as_deref() == Some("1")
}

pub async fn probe_targets(
    targets: Vec<crate::models::ProbeTarget>,
) -> Result<Vec<ProbeEvent>, String> {
    let mut jobs = Vec::with_capacity(targets.len());
    for target in targets {
        jobs.push(tokio::spawn(async move {
            let probe =
                crate::probe::tcp_probe(&target.host, target.port, Duration::from_millis(800))
                    .await;
            ProbeEvent {
                id: target.id,
                online: probe.online,
                latency_ms: probe.latency_ms,
            }
        }));
    }
    let mut events = Vec::with_capacity(jobs.len());
    for job in jobs {
        events.push(job.await.map_err(|err| format!("探测任务失败: {err}"))?);
    }
    Ok(events)
}

pub fn apply_autostart(app: &AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let launch = app.autolaunch();
    if enabled {
        launch
            .enable()
            .map_err(|err| format!("设置开机自启失败: {err}"))
    } else if launch.is_enabled().unwrap_or(false) {
        launch
            .disable()
            .map_err(|err| format!("关闭开机自启失败: {err}"))
    } else {
        Ok(())
    }
}

pub fn emit_probe(app: &AppHandle, event: &ProbeEvent) {
    let _ = app.emit("machine-probe", event);
}
