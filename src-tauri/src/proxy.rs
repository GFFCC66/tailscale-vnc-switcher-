//! WebSocket 到裸 TCP 的桥。
//!
//! VNC 服务器（x11vnc、TigerVNC）说的是 TCP 上的 RFB。noVNC 只能走 WebSocket。
//! 每台机器在 `127.0.0.1` 上占一个临时端口：浏览器连上来之后，这里再拨
//! Tailscale IP 的 VNC 端口，双向搬运二进制帧。不监听外网地址。
//!
//! 切换桌面时前端只是显隐画面，不会新建 WebSocket，因此这里的 TCP 计数保持不变。
//! 手动重连会新开一条 WebSocket，计数加一。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::Message;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyInfo {
    pub id: String,
    pub ws_url: String,
    pub local_port: u16,
    pub target_host: String,
    pub target_port: u16,
    pub tcp_connections: u64,
}

struct ProxySlot {
    host: String,
    port: u16,
    local_port: u16,
    shutdown: watch::Sender<bool>,
    task: JoinHandle<()>,
    clients: Arc<Mutex<Vec<JoinHandle<()>>>>,
    tcp_connections: Arc<AtomicU64>,
}

impl ProxySlot {
    fn info(&self, id: &str) -> ProxyInfo {
        ProxyInfo {
            id: id.to_string(),
            ws_url: format!("ws://127.0.0.1:{}", self.local_port),
            local_port: self.local_port,
            target_host: self.host.clone(),
            target_port: self.port,
            tcp_connections: self.tcp_connections.load(Ordering::Relaxed),
        }
    }

    fn stop(&self) {
        let _ = self.shutdown.send(true);
        self.task.abort();
        if let Ok(clients) = self.clients.lock() {
            for client in clients.iter() {
                client.abort();
            }
        }
    }
}

#[derive(Default)]
pub struct ProxyManager {
    slots: Mutex<HashMap<String, ProxySlot>>,
}

impl ProxyManager {
    pub fn new() -> Self {
        Self {
            slots: Mutex::new(HashMap::new()),
        }
    }

    /// 同一台机器、同一个目标已经在听，就复用本地端口。
    pub async fn ensure(&self, id: &str, host: &str, port: u16) -> Result<ProxyInfo, String> {
        let host = crate::probe::normalize_host(host);
        if host.is_empty() {
            return Err("主机地址是空的".into());
        }
        if port == 0 {
            return Err("端口不能是 0".into());
        }
        if let Some(info) = self.reusable(id, &host, port) {
            return Ok(info);
        }
        self.spawn(id, host, port).await
    }

    pub fn stop(&self, id: &str) {
        let mut slots = self.slots.lock().expect("proxy map poisoned");
        if let Some(slot) = slots.remove(id) {
            slot.stop();
        }
    }

    pub fn info(&self, id: &str) -> Option<ProxyInfo> {
        let slots = self.slots.lock().expect("proxy map poisoned");
        slots.get(id).map(|slot| slot.info(id))
    }

    pub fn tcp_count(&self, id: &str) -> u64 {
        self.info(id).map(|info| info.tcp_connections).unwrap_or(0)
    }

    pub fn tcp_counts(&self) -> HashMap<String, u64> {
        let slots = self.slots.lock().expect("proxy map poisoned");
        slots
            .iter()
            .map(|(id, slot)| (id.clone(), slot.tcp_connections.load(Ordering::Relaxed)))
            .collect()
    }

    fn reusable(&self, id: &str, host: &str, port: u16) -> Option<ProxyInfo> {
        let slots = self.slots.lock().expect("proxy map poisoned");
        let slot = slots.get(id)?;
        if slot.host == host && slot.port == port && !slot.task.is_finished() {
            Some(slot.info(id))
        } else {
            None
        }
    }

    async fn spawn(&self, id: &str, host: String, port: u16) -> Result<ProxyInfo, String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|err| format!("绑定本地代理端口失败: {err}"))?;
        let local_port = listener
            .local_addr()
            .map_err(|err| format!("读取本地代理端口失败: {err}"))?
            .port();
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let tcp_connections = Arc::new(AtomicU64::new(0));
        let clients = Arc::new(Mutex::new(Vec::new()));
        let task = tokio::spawn(serve(
            listener,
            host.clone(),
            port,
            shutdown_rx,
            Arc::clone(&tcp_connections),
            Arc::clone(&clients),
        ));
        let slot = ProxySlot {
            host,
            port,
            local_port,
            shutdown: shutdown_tx,
            task,
            clients,
            tcp_connections,
        };
        let info = slot.info(id);
        let mut slots = self.slots.lock().expect("proxy map poisoned");
        if let Some(old) = slots.insert(id.to_string(), slot) {
            old.stop();
        }
        Ok(info)
    }
}

async fn serve(
    listener: TcpListener,
    host: String,
    port: u16,
    mut shutdown: watch::Receiver<bool>,
    tcp_connections: Arc<AtomicU64>,
    clients: Arc<Mutex<Vec<JoinHandle<()>>>>,
) {
    loop {
        tokio::select! {
            changed = shutdown.changed() => {
                if changed.is_err() || *shutdown.borrow() {
                    break;
                }
            }
            accepted = listener.accept() => {
                let (stream, _) = match accepted {
                    Ok(pair) => pair,
                    Err(err) => {
                        log::warn!("本地代理 accept 失败: {err}");
                        break;
                    }
                };
                let host = host.clone();
                let counter = Arc::clone(&tcp_connections);
                let handle = tokio::spawn(async move {
                    if let Err(err) = bridge_client(stream, &host, port, &counter).await {
                        log::info!("VNC 桥接结束: {err}");
                    }
                });
                if let Ok(mut list) = clients.lock() {
                    list.retain(|item| !item.is_finished());
                    list.push(handle);
                }
            }
        }
    }
}

#[allow(clippy::result_large_err)] // tungstenite 的握手回调把整个 HTTP 响应放在 Err 里
async fn bridge_client(
    client: TcpStream,
    host: &str,
    port: u16,
    counter: &AtomicU64,
) -> Result<(), String> {
    let _ = client.set_nodelay(true);
    let callback = |request: &Request, mut response: Response| -> Result<Response, ErrorResponse> {
        if let Some(raw) = request.headers().get("Sec-WebSocket-Protocol") {
            if let Ok(value) = raw.to_str() {
                let wants_binary = value
                    .split(',')
                    .any(|part| part.trim().eq_ignore_ascii_case("binary"));
                if wants_binary {
                    if let Ok(header) = "binary".parse() {
                        response
                            .headers_mut()
                            .insert("Sec-WebSocket-Protocol", header);
                    }
                }
            }
        }
        Ok(response)
    };

    let websocket = tokio_tungstenite::accept_hdr_async(client, callback)
        .await
        .map_err(|err| format!("WebSocket 握手失败: {err}"))?;

    let upstream = tokio::time::timeout(Duration::from_secs(8), TcpStream::connect((host, port)))
        .await
        .map_err(|_| format!("连接 {host}:{port} 超时"))?
        .map_err(|err| format!("连接 {host}:{port} 失败: {err}"))?;
    let _ = upstream.set_nodelay(true);
    counter.fetch_add(1, Ordering::Relaxed);

    let (mut ws_sink, mut ws_stream) = websocket.split();
    let (mut tcp_read, mut tcp_write) = upstream.into_split();

    let to_server = async {
        while let Some(message) = ws_stream.next().await {
            let message = message.map_err(|err| err.to_string())?;
            match message {
                Message::Binary(data) => {
                    tcp_write
                        .write_all(&data)
                        .await
                        .map_err(|err| err.to_string())?;
                }
                Message::Text(text) => {
                    tcp_write
                        .write_all(text.as_bytes())
                        .await
                        .map_err(|err| err.to_string())?;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
        let _ = tcp_write.shutdown().await;
        Ok::<(), String>(())
    };

    let to_client = async {
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            let read = tcp_read
                .read(&mut buffer)
                .await
                .map_err(|err| err.to_string())?;
            if read == 0 {
                break;
            }
            ws_sink
                .send(Message::Binary(buffer[..read].to_vec().into()))
                .await
                .map_err(|err| err.to_string())?;
        }
        let _ = ws_sink.close().await;
        Ok::<(), String>(())
    };

    tokio::select! {
        result = to_server => result,
        result = to_client => result,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    async fn fake_vnc(payload_expect: &'static [u8], reply: &'static [u8]) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut got = vec![0u8; payload_expect.len()];
            socket.read_exact(&mut got).await.unwrap();
            assert_eq!(got, payload_expect);
            socket.write_all(reply).await.unwrap();
            // 留着连接，直到对端关掉，避免测试还在读时这里先退出。
            let mut drain = [0u8; 8];
            let _ = socket.read(&mut drain).await;
        });
        port
    }

    #[tokio::test]
    async fn bridges_both_directions_and_counts_one_tcp() {
        let banner = b"RFB 003.008\n";
        let port = fake_vnc(b"CLIENT", banner).await;
        let manager = ProxyManager::new();
        let info = manager.ensure("cue", "127.0.0.1", port).await.unwrap();
        let again = manager.ensure("cue", "127.0.0.1", port).await.unwrap();
        assert_eq!(info.local_port, again.local_port, "同一目标应复用代理");

        let (mut socket, _) = tokio_tungstenite::connect_async(&info.ws_url)
            .await
            .unwrap();
        socket
            .send(Message::Binary(b"CLIENT".to_vec().into()))
            .await
            .unwrap();
        let incoming = socket.next().await.unwrap().unwrap();
        match incoming {
            Message::Binary(data) => assert_eq!(&data[..], banner),
            other => panic!("期望二进制 RFB 横幅，得到 {other:?}"),
        }
        assert_eq!(manager.tcp_count("cue"), 1);

        // 再确保一次不应新开到 VNC 的 TCP。计数仍然是 1。
        let _ = manager.ensure("cue", "127.0.0.1", port).await.unwrap();
        assert_eq!(manager.tcp_count("cue"), 1);
        manager.stop("cue");
        assert_eq!(manager.tcp_count("cue"), 0);
    }

    #[tokio::test]
    async fn accepts_novnc_binary_subprotocol() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            socket.write_all(b"RFB 003.008\n").await.unwrap();
            let mut drain = [0u8; 4];
            let _ = socket.read(&mut drain).await;
        });
        let manager = ProxyManager::new();
        let info = manager.ensure("box", "[127.0.0.1]", port).await.unwrap();

        let mut request = info.ws_url.into_client_request().unwrap();
        request
            .headers_mut()
            .insert("Sec-WebSocket-Protocol", "binary".parse().unwrap());
        let (mut socket, response) = tokio_tungstenite::connect_async(request).await.unwrap();
        let protocol = response
            .headers()
            .get("Sec-WebSocket-Protocol")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("");
        assert_eq!(protocol, "binary");
        let incoming = socket.next().await.unwrap().unwrap();
        match incoming {
            Message::Binary(data) => assert!(data.starts_with(b"RFB ")),
            other => panic!("unexpected {other:?}"),
        }
        manager.stop("box");
    }

    #[tokio::test]
    async fn moves_a_large_buffer() {
        let blob = vec![0x5au8; 200_000];
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let expected = blob.clone();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut got = vec![0u8; expected.len()];
            socket.read_exact(&mut got).await.unwrap();
            assert_eq!(got, expected);
            socket.write_all(&got).await.unwrap();
        });
        let manager = ProxyManager::new();
        let info = manager.ensure("big", "127.0.0.1", port).await.unwrap();
        let (mut socket, _) = tokio_tungstenite::connect_async(&info.ws_url)
            .await
            .unwrap();
        socket
            .send(Message::Binary(blob.clone().into()))
            .await
            .unwrap();
        let mut collected = Vec::new();
        while collected.len() < blob.len() {
            let message = socket.next().await.unwrap().unwrap();
            match message {
                Message::Binary(data) => collected.extend_from_slice(&data),
                other => panic!("unexpected {other:?}"),
            }
        }
        assert_eq!(collected, blob);
        manager.stop("big");
    }

    /// 设置 `VNC_TEST_ADDR=127.0.0.1:5900` 时，对真实的 x11vnc / TigerVNC 读 RFB 横幅。
    /// 平时不设置，测试直接通过，避免单元测试依赖显示器。
    #[tokio::test]
    async fn real_vnc_banner_when_configured() {
        let Ok(addr) = std::env::var("VNC_TEST_ADDR") else {
            return;
        };
        let (host, port_text) = addr
            .rsplit_once(':')
            .expect("VNC_TEST_ADDR 需要是 host:port");
        let port: u16 = port_text.parse().expect("端口");
        let manager = ProxyManager::new();
        let info = manager.ensure("real", host, port).await.expect("代理");
        let (mut socket, _) = tokio_tungstenite::connect_async(&info.ws_url)
            .await
            .expect("websocket");
        let message = socket.next().await.expect("帧").expect("消息");
        match message {
            Message::Binary(data) => {
                let text = String::from_utf8_lossy(&data);
                assert!(text.starts_with("RFB "), "真实服务器横幅异常: {text}");
            }
            other => panic!("期望二进制 RFB 横幅，得到 {other:?}"),
        }
        assert_eq!(manager.tcp_count("real"), 1);
        manager.stop("real");
    }
}
