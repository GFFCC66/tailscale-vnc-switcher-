# 云桌面切换

把几台通过 Tailscale 能访问的云电脑放进同一个窗口。会话连上之后留在后台，切换时只是显示或藏起画面，不会重新握手。

界面默认是暗色。目标平台是 Windows，同一套代码也可以在 Linux 和 macOS 上编译。

## 能做什么

- 左侧固定机器列表：名称、Tailscale 地址、端口、在线或离线、连接状态。可以添加、编辑、删除，也可以分组和打标签。
- 每台机器一块 noVNC 画面。已经连上的会话都留着，切换不会重连。
- 画面按窗口自适应缩放，也可以改成原始分辨率。支持全屏。
- 快捷键：`Ctrl+1` … `Ctrl+9` 跳到对应机器，`Ctrl+Tab` / `Ctrl+Shift+Tab` 在已经打开的会话之间循环。切换时有一段很短的过渡。
- VNC 密码可选，用 AES-256-GCM 加密后存在本机。
- 断线自动重连，也可以单独断开或重连某一台。
- 用一次短 TCP 连接估计延迟，并显示极佳 / 良好 / 一般 / 较差。
- 底部状态栏：当前机器、分辨率、已连接时长、延迟。
- 设置：快捷键、主题（默认暗色）、开机自启、缩放和画质。

第一次打开会放进四台示例，地址是占位的 Tailscale IP，需要改成真实地址才能连上：

| 名称 | 地址 | 端口 | 类型 |
| --- | --- | --- | --- |
| Cue | 100.64.1.10 | 5900 | x11vnc |
| Instabox | 100.64.1.11 | 5900 | 原生 stream，默认仍走 VNC |
| Simular Box | 100.64.1.12 | 5900 | 可改成网页 live view |
| 阿里云 Ubuntu | 100.64.1.13 | 5901 | TigerVNC |

## 为什么需要一座桥

x11vnc 和 TigerVNC 说的是 TCP 上的 RFB。noVNC 只能走 WebSocket。应用在 `127.0.0.1` 上为每台机器开一个临时端口：页面连这个 WebSocket，Rust 侧再去拨 Tailscale 地址上的 VNC 端口，双向搬运二进制帧。代理不监听外网。

如果设备自己已经提供 WebSocket，把传输改成「已有 WebSocket」并填写 `ws://` 或 `wss://`。网页 live view 则走 iframe。

## 目录

| 路径 | 作用 |
| --- | --- |
| `src/composables/useSessions.ts` | 多会话。`switchTo` 只改当前显示的机器，不新建 RFB |
| `src/components/SessionStage.vue` | 所有已打开的画面叠在一起，用透明度隐藏 |
| `src/components/Sidebar.vue` | 机器列表、分组、在线状态 |
| `src/App.vue` | 快捷键和全屏 |
| `src-tauri/src/proxy.rs` | WebSocket 到 TCP 的桥，以及它的测试 |
| `src-tauri/src/store.rs` | `config.json` 的读写、示例机器、校验 |
| `src-tauri/src/crypto.rs` | 密码的 AES-256-GCM |
| `src-tauri/src/probe.rs` | 在线状态和延迟 |
| `src-tauri/src/commands.rs` | 给前端调用的命令 |
| `scripts/e2e-linux.sh` | 拉起两台本地 VNC 做验证 |
| `.github/workflows/windows-installer.yml` | 打 Windows 安装包 |

配置文件在应用配置目录里的 `config.json`，密钥是旁边的 `secret.key`。设置页底部会写出当前路径。Linux 上通常是 `~/.config/app.vncswitcher.desktop/`。

## 本地运行

需要 Node.js 22、Rust stable，以及 Tauri 的系统依赖。Linux 上主要是 `libwebkit2gtk-4.1-dev`、`libgtk-3-dev`、`libsoup-3.0-dev`、`patchelf`。完整列表见 [Tauri 前置条件](https://v2.tauri.app/start/prerequisites/)。

```bash
npm install
npm run tauri dev
```

只检查前端：

```bash
npm test
npm run build
```

只检查 Rust：

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

`cargo test` 不需要显示器。想再对真实的 VNC 读一次协议横幅时，先跑 `scripts/e2e-linux.sh`，再按它打印的命令带上 `VNC_TEST_ADDR`。

## 打 Windows 安装包

在一台 Windows 机器上安装 Visual Studio 的 C++ 生成工具、WebView2（Win10/Win11 通常已经有）以及上面的 Node 和 Rust，然后：

```bash
npm install
npm run tauri build
```

NSIS 安装包在 `src-tauri/target/release/bundle/nsis/`。

仓库里的 GitHub Actions 会在打 `v*` 标签，或在 Actions 页面手动运行 “Windows installer” 时，用 `tauri-action` 打出同一个安装包并上传为构建产物。

## 添加一台机器

1. 点侧边栏右上角的加号。
2. 填写名称、Tailscale IP 或 MagicDNS 主机名、端口。x11vnc 常用 5900，TigerVNC 常用 5901。
3. 类型只影响列表上的标记。真正决定怎么连的是「传输」：
   - VNC：走本机 WebSocket 桥，再拨 TCP。
   - 已有 WebSocket：填写设备自己的 `ws://` 或 `wss://`。
   - 网页：填写 live view 的 `http://` 或 `https://`。
4. 需要密码就填上。留空表示这次不改已保存的密码；勾选清除则会忘掉它。
5. 保存后点这台机器，或按它在列表里的序号（`Ctrl+1` 是第一台）。

也可以直接改 `config.json`。密码字段是 `passwordEnc`，不要自己写明文；从界面保存才会加密。

## 验证过的行为

在 Linux 上用两块 Xvfb 各跑一个 x11vnc（一台无密码，一台 VNC 密码），应用同时连上，分辨率都是 1280×800。之后用 `Ctrl+1`、`Ctrl+4`、`Ctrl+Tab` 和侧边栏点击来回切换。两台会话的连接代号一直是 1，代理上的 TCP 连接数也一直是 1，状态栏里的连接时长没有归零。点开尚未连接的占位机器才会新开连接。

## 限制

- 不包含 Tailscale 本身。这台电脑需要已经登录 tailnet，应用只去拨写好的地址。
- 密码密钥放在配置目录，不是操作系统钥匙串。把整个目录拷走就能解密。
- 快捷键只在窗口聚焦时生效，不会注册成全局热键。
- 网页 live view 只是一个 iframe，对方如果禁止嵌入就打不开。
- 自适应缩放保持画面比例，窗口比例不同时上下或左右会留边。
- 没有声音、剪贴板同步和文件传输。
- 开机自启走 Tauri 的 autostart 插件：Windows 注册表、Linux 的 `.desktop`、macOS 的 LaunchAgent。
- 这里没有实际产出 Windows 安装包，安装包要在 Windows 或上面的 Actions 里打。
