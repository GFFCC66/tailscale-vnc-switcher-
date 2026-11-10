#!/usr/bin/env bash
# 在本机拉起两台 VNC，供云桌面切换器做端到端验证。
# 不会占用已经在用的桌面。两块虚拟屏幕分别是 :2 和 :3。
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
port_a="${VNC_PORT_A:-5910}"
port_b="${VNC_PORT_B:-5911}"
password="${VNC_E2E_PASSWORD:-e2e-secret}"

for cmd in Xvfb x11vnc xsetroot python3; do
  command -v "$cmd" >/dev/null || { echo "缺少命令: $cmd" >&2; exit 1; }
done

if [[ ! -S /tmp/.X11-unix/X2 ]]; then
  Xvfb :2 -screen 0 1280x800x24 -ac +extension RENDER -noreset >/tmp/xvfb-2.log 2>&1 &
fi
if [[ ! -S /tmp/.X11-unix/X3 ]]; then
  Xvfb :3 -screen 0 1280x800x24 -ac +extension RENDER -noreset >/tmp/xvfb-3.log 2>&1 &
fi
for _ in 1 2 3 4 5 6 7 8 9 10; do
  [[ -S /tmp/.X11-unix/X2 && -S /tmp/.X11-unix/X3 ]] && break
  sleep 0.2
done

DISPLAY=:2 xsetroot -solid "#12382f"
DISPLAY=:3 xsetroot -solid "#3a2a14"
DISPLAY=:2 DESKTOP_TITLE=$'Cue\n会话 A' DESKTOP_COLOR="#12382f" \
  DESKTOP_SUB=$'本地 x11vnc\n切换时这条连接保持着' \
  python3 "$root/scripts/demo-desktop.py" >/tmp/desk-a.log 2>&1 &
DISPLAY=:3 DESKTOP_TITLE=$'阿里云 Ubuntu\n会话 B' DESKTOP_COLOR="#3a2a14" \
  DESKTOP_SUB=$'已保存密码的会话\n切换后仍然是同一条连接' \
  python3 "$root/scripts/demo-desktop.py" >/tmp/desk-b.log 2>&1 &
sleep 0.4

x11vnc -display :2 -rfbport "$port_a" -localhost -forever -shared -nopw -bg -noxdamage -o /tmp/x11vnc-a.log
x11vnc -display :3 -rfbport "$port_b" -localhost -forever -shared -passwd "$password" -bg -noxdamage -o /tmp/x11vnc-b.log

cat <<EOF
两台本地 VNC 已就绪：
  127.0.0.1:${port_a}  无密码
  127.0.0.1:${port_b}  密码 ${password}

在另一个终端启动应用（会自动连上这两台，并把报告写到 /tmp/vnc-switcher-e2e.json）：

  cd ${root}
  VNC_SWITCHER_E2E=1 \\
  VNC_SWITCHER_E2E_PORTS=${port_a},${port_b} \\
  VNC_SWITCHER_E2E_PASSWORD=${password} \\
  VNC_SWITCHER_CONFIG_DIR=/tmp/vnc-switcher-e2e-config \\
  npm run tauri dev

只验证 WebSocket 桥是否读到真实 RFB 横幅：

  VNC_TEST_ADDR=127.0.0.1:${port_a} cargo test --manifest-path src-tauri/Cargo.toml real_vnc_banner -- --nocapture
EOF
