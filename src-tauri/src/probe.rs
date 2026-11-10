//! 用一次短 TCP 连接估计机器是否在线以及大致延迟。
//!
//! 这不是 VNC 握手时延，只回答“这个地址的端口现在通不通”。
//! 探测走直连，不经过 WebSocket 代理，所以不会被算进会话的 TCP 计数。

use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::net::TcpStream;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    pub online: bool,
    pub latency_ms: Option<u64>,
}

pub async fn tcp_probe(host: &str, port: u16, timeout: Duration) -> Probe {
    let host = normalize_host(host);
    if host.is_empty() || port == 0 {
        return Probe {
            online: false,
            latency_ms: None,
        };
    }
    let started = Instant::now();
    match tokio::time::timeout(timeout, TcpStream::connect((host.as_str(), port))).await {
        Ok(Ok(_stream)) => Probe {
            online: true,
            latency_ms: Some(started.elapsed().as_millis() as u64),
        },
        _ => Probe {
            online: false,
            latency_ms: None,
        },
    }
}

pub fn normalize_host(host: &str) -> String {
    host.trim().trim_matches(['[', ']']).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    #[tokio::test]
    async fn online_listener_and_closed_port() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let probe = tcp_probe("127.0.0.1", port, Duration::from_secs(1)).await;
        assert!(probe.online);
        assert!(probe.latency_ms.is_some());

        drop(listener);
        let closed = tcp_probe("127.0.0.1", port, Duration::from_millis(500)).await;
        assert!(!closed.online);
        assert!(closed.latency_ms.is_none());
    }

    #[test]
    fn strips_ipv6_brackets() {
        assert_eq!(normalize_host(" [::1] "), "::1");
        assert_eq!(normalize_host("100.64.1.10"), "100.64.1.10");
    }
}
