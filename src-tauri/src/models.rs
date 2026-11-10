//! 机器、设置和探测结果的数据模型。
//!
//! JSON 使用 camelCase，和前端 TypeScript 类型一一对应。
//! 密码永远不出现在对外结构里，只以密文 `passwordEnc` 留在磁盘上。

use serde::{Deserialize, Serialize};

pub const DATA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Machine {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    /// `vnc-tcp`：本机 WebSocket 代理再拨 TCP。
    /// `websocket`：直接连 `directUrl` 上已有的 WebSocket（例如设备自带 websockify）。
    /// `web`：用 iframe 打开网页 live view。
    pub transport: String,
    /// 展示用的机器类型：`x11vnc` / `tigervnc` / `stream` / `web-live`。
    pub kind: String,
    pub group: String,
    pub tags: Vec<String>,
    pub notes: String,
    pub direct_url: Option<String>,
    pub view_only: bool,
    pub password_enc: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// 返回给前端的机器。没有密文，只有“是否已保存密码”。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MachinePublic {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub transport: String,
    pub kind: String,
    pub group: String,
    pub tags: Vec<String>,
    pub notes: String,
    pub direct_url: Option<String>,
    pub view_only: bool,
    pub has_password: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl Machine {
    pub fn to_public(&self) -> MachinePublic {
        MachinePublic {
            id: self.id.clone(),
            name: self.name.clone(),
            host: self.host.clone(),
            port: self.port,
            transport: self.transport.clone(),
            kind: self.kind.clone(),
            group: self.group.clone(),
            tags: self.tags.clone(),
            notes: self.notes.clone(),
            direct_url: self.direct_url.clone(),
            view_only: self.view_only,
            has_password: self.password_enc.is_some(),
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineInput {
    pub id: Option<String>,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub transport: String,
    pub kind: String,
    pub group: String,
    pub tags: Vec<String>,
    pub notes: String,
    pub direct_url: Option<String>,
    pub view_only: bool,
    /// `None` 表示编辑时不改密码；`Some("")` 表示清除；其他值表示设置新密码。
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Hotkeys {
    pub next: String,
    pub previous: String,
    pub fullscreen: String,
    pub reconnect: String,
    /// 与数字 1–9 组合，例如 "Ctrl" 表示 Ctrl+1 … Ctrl+9。
    pub jump_modifier: String,
}

impl Default for Hotkeys {
    fn default() -> Self {
        Self {
            next: "Ctrl+Tab".into(),
            previous: "Ctrl+Shift+Tab".into(),
            fullscreen: "F11".into(),
            reconnect: "Ctrl+Shift+R".into(),
            jump_modifier: "Ctrl".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// `dark` 或 `light`。默认暗色。
    pub theme: String,
    pub autostart: bool,
    pub auto_reconnect: bool,
    pub reconnect_interval_ms: u64,
    /// `fit` 自适应缩放，`native` 原始分辨率加滚动。
    pub scale_mode: String,
    /// noVNC 画质 0–9。
    pub quality_level: u8,
    pub show_dot_cursor: bool,
    pub hotkeys: Hotkeys,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "dark".into(),
            autostart: false,
            auto_reconnect: true,
            reconnect_interval_ms: 3000,
            scale_mode: "fit".into(),
            quality_level: 6,
            show_dot_cursor: true,
            hotkeys: Hotkeys::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub version: u32,
    #[serde(default)]
    pub machines: Vec<Machine>,
    #[serde(default)]
    pub settings: Settings,
}

impl Default for AppData {
    fn default() -> Self {
        Self {
            version: DATA_VERSION,
            machines: Vec::new(),
            settings: Settings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectResponse {
    pub transport: String,
    pub ws_url: Option<String>,
    pub web_url: Option<String>,
    /// 仅在发起连接时解密一次，列表接口不会带上它。
    pub password: Option<String>,
    pub view_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProbeEvent {
    pub id: String,
    pub online: bool,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ProbeTarget {
    pub id: String,
    pub host: String,
    pub port: u16,
}
