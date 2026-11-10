//! 机器列表和设置的本地 JSON 存储。
//!
//! 写入先落到 `config.json.tmp` 再改名，避免写到一半崩溃把配置截断。
//! 第一次打开且没有文件时，放入说明文档里的四台示例机器（地址是占位 Tailscale IP）。

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use uuid::Uuid;

use crate::crypto::Vault;
use crate::models::{
    AppData, ConnectResponse, Machine, MachineInput, MachinePublic, ProbeTarget, Settings,
    DATA_VERSION,
};
use crate::probe::normalize_host;

pub struct Store {
    path: PathBuf,
    vault: Vault,
    data: AppData,
}

impl Store {
    pub fn open(dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(dir).map_err(|err| format!("创建配置目录失败: {err}"))?;
        let path = dir.join("config.json");
        let vault = Vault::load_or_create(&dir.join("secret.key"))?;
        let existed = path.exists();
        let data = if existed {
            let text = fs::read_to_string(&path).map_err(|err| format!("读取配置失败: {err}"))?;
            serde_json::from_str::<AppData>(&text)
                .map_err(|err| format!("配置文件损坏，没有自动覆盖: {err}"))?
        } else {
            AppData {
                version: DATA_VERSION,
                machines: example_machines(),
                settings: Settings::default(),
            }
        };
        let store = Self { path, vault, data };
        if !existed {
            store.persist()?;
        }
        Ok(store)
    }

    pub fn config_path(&self) -> &Path {
        &self.path
    }

    pub fn machines(&self) -> Vec<MachinePublic> {
        self.data.machines.iter().map(Machine::to_public).collect()
    }

    pub fn settings(&self) -> Settings {
        self.data.settings.clone()
    }

    pub fn probe_targets(&self) -> Vec<ProbeTarget> {
        self.data
            .machines
            .iter()
            .filter(|machine| machine.transport != "web" || machine.port != 0)
            .map(|machine| ProbeTarget {
                id: machine.id.clone(),
                host: machine.host.clone(),
                port: machine.port,
            })
            .collect()
    }

    pub fn upsert(&mut self, input: MachineInput) -> Result<MachinePublic, String> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err("请填写机器名称".into());
        }
        if name.chars().count() > 64 {
            return Err("机器名称最长 64 个字符".into());
        }
        let host = normalize_host(&input.host);
        if host.is_empty() || host.len() > 255 || host.contains(char::is_whitespace) {
            return Err("主机地址无效".into());
        }
        if input.port == 0 {
            return Err("端口需要在 1 到 65535 之间".into());
        }
        let transport = input.transport.trim().to_string();
        if !matches!(transport.as_str(), "vnc-tcp" | "websocket" | "web") {
            return Err("传输方式只能是 vnc-tcp、websocket 或 web".into());
        }
        let kind = normalize_kind(&input.kind);
        let direct_url = normalize_direct_url(input.direct_url.as_deref(), &transport)?;
        let group = input.group.trim().chars().take(32).collect::<String>();
        let notes = input.notes.trim().chars().take(500).collect::<String>();
        let tags = normalize_tags(input.tags);
        let now = Utc::now().to_rfc3339();

        if let Some(id) = input
            .id
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            let machine = self
                .data
                .machines
                .iter_mut()
                .find(|machine| machine.id == id)
                .ok_or_else(|| "找不到要编辑的机器".to_string())?;
            machine.name = name;
            machine.host = host;
            machine.port = input.port;
            machine.transport = transport;
            machine.kind = kind;
            machine.group = group;
            machine.tags = tags;
            machine.notes = notes;
            machine.direct_url = direct_url;
            machine.view_only = input.view_only;
            machine.updated_at = now;
            if let Some(password) = input.password {
                if password.is_empty() {
                    machine.password_enc = None;
                } else {
                    validate_password(&password)?;
                    machine.password_enc = Some(self.vault.encrypt(&password)?);
                }
            }
            let public = machine.to_public();
            self.persist()?;
            return Ok(public);
        }

        let password_enc = match input.password.as_deref() {
            Some(password) if !password.is_empty() => {
                validate_password(password)?;
                Some(self.vault.encrypt(password)?)
            }
            _ => None,
        };
        let machine = Machine {
            id: Uuid::new_v4().to_string(),
            name,
            host,
            port: input.port,
            transport,
            kind,
            group,
            tags,
            notes,
            direct_url,
            view_only: input.view_only,
            password_enc,
            created_at: now.clone(),
            updated_at: now,
        };
        let public = machine.to_public();
        self.data.machines.push(machine);
        self.persist()?;
        Ok(public)
    }

    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        let before = self.data.machines.len();
        self.data.machines.retain(|machine| machine.id != id);
        if self.data.machines.len() == before {
            return Err("找不到要删除的机器".into());
        }
        self.persist()
    }

    pub fn save_settings(&mut self, mut settings: Settings) -> Result<Settings, String> {
        settings.theme = match settings.theme.as_str() {
            "light" => "light".into(),
            _ => "dark".into(),
        };
        settings.scale_mode = match settings.scale_mode.as_str() {
            "native" => "native".into(),
            _ => "fit".into(),
        };
        settings.reconnect_interval_ms = settings.reconnect_interval_ms.clamp(1_000, 60_000);
        settings.quality_level = settings.quality_level.min(9);
        if settings.hotkeys.next.trim().is_empty()
            || settings.hotkeys.previous.trim().is_empty()
            || settings.hotkeys.jump_modifier.trim().is_empty()
        {
            return Err("快捷键不能为空".into());
        }
        self.data.settings = settings.clone();
        self.persist()?;
        Ok(settings)
    }

    pub fn connect_plan(&self, id: &str) -> Result<ConnectResponse, String> {
        let machine = self
            .data
            .machines
            .iter()
            .find(|machine| machine.id == id)
            .ok_or_else(|| "找不到这台机器".to_string())?;
        let password = match &machine.password_enc {
            Some(encoded) => Some(self.vault.decrypt(encoded)?),
            None => None,
        };
        Ok(ConnectResponse {
            transport: machine.transport.clone(),
            ws_url: None,
            web_url: if machine.transport == "web" {
                machine.direct_url.clone()
            } else {
                None
            },
            password,
            view_only: machine.view_only,
        })
    }

    pub fn endpoint(&self, id: &str) -> Result<(String, String, u16, Option<String>), String> {
        let machine = self
            .data
            .machines
            .iter()
            .find(|machine| machine.id == id)
            .ok_or_else(|| "找不到这台机器".to_string())?;
        Ok((
            machine.transport.clone(),
            machine.host.clone(),
            machine.port,
            machine.direct_url.clone(),
        ))
    }

    /// 端到端验证时换成可控的本机 VNC，避免去拨占位 Tailscale 地址。
    pub fn install_e2e_fixtures(
        &mut self,
        ports: &[u16],
        password: Option<&str>,
    ) -> Result<(), String> {
        let now = Utc::now().to_rfc3339();
        let mut machines = Vec::new();
        let specs = [
            (
                "e2e-cue",
                "Cue",
                0usize,
                "x11vnc",
                "云电脑",
                "Cue 的本地验证桌面。",
            ),
            (
                "e2e-instabox",
                "Instabox",
                usize::MAX,
                "stream",
                "云电脑",
                "Instabox 示例。这里仍是占位 Tailscale 地址，用来展示离线状态。",
            ),
            (
                "e2e-simular",
                "Simular Box",
                usize::MAX,
                "web-live",
                "云电脑",
                "Simular Box 示例，占位地址。",
            ),
            (
                "e2e-aliyun",
                "阿里云 Ubuntu",
                1usize,
                "tigervnc",
                "云主机",
                "阿里云 Ubuntu 的本地验证桌面。",
            ),
        ];
        for (index, (id, name, port_index, kind, group, notes)) in specs.iter().enumerate() {
            let (host, port, password_enc) = if *port_index == usize::MAX {
                let host = if *id == "e2e-instabox" {
                    "100.64.1.11"
                } else {
                    "100.64.1.12"
                };
                (host.to_string(), 5900, None)
            } else {
                let port = *ports.get(*port_index).ok_or("E2E 端口不足")?;
                let password_enc = if *port_index == 1 {
                    match password {
                        Some(value) if !value.is_empty() => Some(self.vault.encrypt(value)?),
                        _ => None,
                    }
                } else {
                    None
                };
                ("127.0.0.1".to_string(), port, password_enc)
            };
            let created = format!("{now}-{index:02}");
            machines.push(Machine {
                id: (*id).to_string(),
                name: (*name).to_string(),
                host,
                port,
                transport: "vnc-tcp".into(),
                kind: (*kind).to_string(),
                group: (*group).to_string(),
                tags: vec!["tailscale".into(), (*kind).to_string()],
                notes: (*notes).to_string(),
                direct_url: None,
                view_only: false,
                password_enc,
                created_at: created.clone(),
                updated_at: created,
            });
        }
        self.data.machines = machines;
        self.data.settings.autostart = false;
        self.persist()
    }

    fn persist(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("创建配置目录失败: {err}"))?;
        }
        let text = serde_json::to_string_pretty(&self.data).map_err(|err| err.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, text + "\n").map_err(|err| format!("写入配置失败: {err}"))?;
        fs::rename(&tmp, &self.path).map_err(|err| format!("替换配置失败: {err}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }
}

fn validate_password(password: &str) -> Result<(), String> {
    if password.chars().count() > 256 {
        return Err("密码最长 256 个字符".into());
    }
    Ok(())
}

fn normalize_kind(kind: &str) -> String {
    match kind.trim() {
        "tigervnc" => "tigervnc".into(),
        "stream" => "stream".into(),
        "web-live" => "web-live".into(),
        _ => "x11vnc".into(),
    }
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for tag in tags {
        let tag = tag
            .trim()
            .trim_matches(',')
            .trim()
            .chars()
            .take(24)
            .collect::<String>();
        if tag.is_empty() || out.iter().any(|existing: &String| existing == &tag) {
            continue;
        }
        out.push(tag);
        if out.len() == 8 {
            break;
        }
    }
    out
}

fn normalize_direct_url(value: Option<&str>, transport: &str) -> Result<Option<String>, String> {
    let Some(raw) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        if matches!(transport, "websocket" | "web") {
            return Err("这种传输方式需要填写直连地址".into());
        }
        return Ok(None);
    };
    let ok = match transport {
        "websocket" => raw.starts_with("ws://") || raw.starts_with("wss://"),
        "web" => raw.starts_with("http://") || raw.starts_with("https://"),
        _ => true,
    };
    if !ok {
        return Err("直连地址和传输方式不匹配".into());
    }
    if raw.chars().count() > 500 {
        return Err("直连地址太长".into());
    }
    Ok(Some(raw.to_string()))
}

fn example_machines() -> Vec<Machine> {
    let base = Utc::now().timestamp();
    let samples = [
        (
            "Cue",
            "100.64.1.10",
            5900u16,
            "x11vnc",
            "云电脑",
            vec!["tailscale", "x11vnc"],
            "Cue。Tailscale IP + 端口 5900，x11vnc。把地址换成这台机器的真实 100.x 地址。",
        ),
        (
            "Instabox",
            "100.64.1.11",
            5900,
            "stream",
            "云电脑",
            vec!["tailscale", "instabox"],
            "Instabox。默认按 VNC 5900 连接。如果设备提供原生 stream 的 WebSocket，把传输改成 WebSocket 并填写直连地址。",
        ),
        (
            "Simular Box",
            "100.64.1.12",
            5900,
            "web-live",
            "云电脑",
            vec!["tailscale", "simular"],
            "Simular Box。可以走 VNC，也可以把传输改成网页，填写 live view 地址。",
        ),
        (
            "阿里云 Ubuntu",
            "100.64.1.13",
            5901,
            "tigervnc",
            "云主机",
            vec!["tailscale", "aliyun"],
            "阿里云 Ubuntu。Tailscale IP + 端口 5901，TigerVNC。",
        ),
    ];
    samples
        .into_iter()
        .enumerate()
        .map(|(index, (name, host, port, kind, group, tags, notes))| {
            let stamp = chrono::DateTime::from_timestamp(base + index as i64, 0)
                .unwrap_or_else(Utc::now)
                .to_rfc3339();
            Machine {
                id: Uuid::new_v4().to_string(),
                name: name.into(),
                host: host.into(),
                port,
                transport: "vnc-tcp".into(),
                kind: kind.into(),
                group: group.into(),
                tags: tags.into_iter().map(str::to_string).collect(),
                notes: notes.into(),
                direct_url: None,
                view_only: false,
                password_enc: None,
                created_at: stamp.clone(),
                updated_at: stamp,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str, password: Option<&str>) -> MachineInput {
        MachineInput {
            id: None,
            name: name.into(),
            host: "100.64.1.20".into(),
            port: 5900,
            transport: "vnc-tcp".into(),
            kind: "x11vnc".into(),
            group: "实验室".into(),
            tags: vec![" a ".into(), "a".into(), "tailscale".into()],
            notes: "备注".into(),
            direct_url: None,
            view_only: false,
            password: password.map(str::to_string),
        }
    }

    #[test]
    fn seeds_examples_and_roundtrips_password_without_plaintext() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        let names: Vec<_> = store
            .machines()
            .into_iter()
            .map(|machine| machine.name)
            .collect();
        assert_eq!(
            names,
            vec!["Cue", "Instabox", "Simular Box", "阿里云 Ubuntu"]
        );
        assert_eq!(store.machines()[3].port, 5901);

        let created = store.upsert(input("工作机", Some("p@ss 词"))).unwrap();
        assert!(created.has_password);
        let plan = store.connect_plan(&created.id).unwrap();
        assert_eq!(plan.password.as_deref(), Some("p@ss 词"));

        let raw = fs::read_to_string(store.config_path()).unwrap();
        assert!(!raw.contains("p@ss"));
        assert!(raw.contains("passwordEnc"));

        let mut edit = input("工作机", None);
        edit.id = Some(created.id.clone());
        edit.host = "[100.64.1.21]".into();
        let updated = store.upsert(edit).unwrap();
        assert_eq!(updated.host, "100.64.1.21");
        assert!(updated.has_password, "留空密码应保持原密文");
        assert_eq!(
            store.connect_plan(&created.id).unwrap().password.as_deref(),
            Some("p@ss 词")
        );

        let mut clear = input("工作机", Some(""));
        clear.id = Some(created.id.clone());
        let cleared = store.upsert(clear).unwrap();
        assert!(!cleared.has_password);

        let mut settings = store.settings();
        settings.theme = "light".into();
        settings.reconnect_interval_ms = 50;
        store.save_settings(settings).unwrap();
        let reopened = Store::open(dir.path()).unwrap();
        assert_eq!(reopened.settings().theme, "light");
        assert_eq!(reopened.settings().reconnect_interval_ms, 1_000);
        assert!(reopened
            .machines()
            .iter()
            .any(|machine| machine.name == "工作机"));

        store.delete(&created.id).unwrap();
        assert!(store.delete(&created.id).is_err());
    }

    #[test]
    fn rejects_bad_input() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(dir.path()).unwrap();
        let mut bad = input("  ", None);
        assert!(store.upsert(bad.clone()).is_err());
        bad.name = "ok".into();
        bad.port = 0;
        assert!(store.upsert(bad).is_err());

        let mut web = input("网页", None);
        web.transport = "web".into();
        web.direct_url = Some("ftp://nope".into());
        assert!(store.upsert(web).is_err());
    }
}
