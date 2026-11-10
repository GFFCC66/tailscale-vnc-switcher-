export type Transport = "vnc-tcp" | "websocket" | "web";
export type MachineKind = "x11vnc" | "tigervnc" | "stream" | "web-live";
export type ThemeName = "dark" | "light";
export type ScaleMode = "fit" | "native";
export type SessionStatus =
  | "connecting"
  | "connected"
  | "reconnecting"
  | "disconnected"
  | "error";

export interface Machine {
  id: string;
  name: string;
  host: string;
  port: number;
  transport: Transport;
  kind: MachineKind;
  group: string;
  tags: string[];
  notes: string;
  directUrl: string | null;
  viewOnly: boolean;
  hasPassword: boolean;
  createdAt: string;
  updatedAt: string;
}

export interface MachineDraft {
  id?: string;
  name: string;
  host: string;
  port: number;
  transport: Transport;
  kind: MachineKind;
  group: string;
  tags: string[];
  notes: string;
  directUrl: string | null;
  viewOnly: boolean;
  /** 留空表示不修改；空字符串表示清除。新建时省略即不保存密码。 */
  password?: string | null;
}

export interface Hotkeys {
  next: string;
  previous: string;
  fullscreen: string;
  reconnect: string;
  jumpModifier: string;
}

export interface Settings {
  theme: ThemeName;
  autostart: boolean;
  autoReconnect: boolean;
  reconnectIntervalMs: number;
  scaleMode: ScaleMode;
  qualityLevel: number;
  showDotCursor: boolean;
  hotkeys: Hotkeys;
}

export interface ConnectResponse {
  transport: Transport;
  wsUrl: string | null;
  webUrl: string | null;
  password: string | null;
  viewOnly: boolean;
}

export interface ProbeEvent {
  id: string;
  online: boolean;
  latencyMs: number | null;
}

export interface LiveSession {
  machineId: string;
  status: SessionStatus;
  /** 每新建一次 RFB 加一。纯切换不会变。 */
  generation: number;
  connectedAt: number | null;
  resolution: { width: number; height: number } | null;
  desktopName: string;
  error: string | null;
  transport: Transport;
  webUrl: string | null;
  manual: boolean;
  attempt: number;
}

export function defaultSettings(): Settings {
  return {
    theme: "dark",
    autostart: false,
    autoReconnect: true,
    reconnectIntervalMs: 3000,
    scaleMode: "fit",
    qualityLevel: 6,
    showDotCursor: true,
    hotkeys: {
      next: "Ctrl+Tab",
      previous: "Ctrl+Shift+Tab",
      fullscreen: "F11",
      reconnect: "Ctrl+Shift+R",
      jumpModifier: "Ctrl",
    },
  };
}
