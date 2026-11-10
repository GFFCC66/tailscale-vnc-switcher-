import type { MachineKind, SessionStatus, Transport } from "../types";

export function errorText(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error && error.message) return error.message;
  return "未知错误";
}

export function kindLabel(kind: MachineKind): string {
  switch (kind) {
    case "tigervnc":
      return "TigerVNC";
    case "stream":
      return "Stream";
    case "web-live":
      return "Live View";
    default:
      return "x11vnc";
  }
}

export function transportLabel(transport: Transport): string {
  switch (transport) {
    case "websocket":
      return "WebSocket";
    case "web":
      return "网页";
    default:
      return "VNC";
  }
}

export function connectionLabel(status: SessionStatus | undefined): string {
  switch (status) {
    case "connecting":
      return "连接中";
    case "connected":
      return "已连接";
    case "reconnecting":
      return "重连中";
    case "error":
      return "失败";
    case "disconnected":
      return "已断开";
    default:
      return "未连接";
  }
}

export function formatDuration(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  const pad = (value: number) => value.toString().padStart(2, "0");
  if (hours > 0) return `${hours}:${pad(minutes)}:${pad(seconds)}`;
  return `${pad(minutes)}:${pad(seconds)}`;
}

export function formatEndpoint(host: string, port: number): string {
  const shown = host.includes(":") ? `[${host}]` : host;
  return `${shown}:${port}`;
}
