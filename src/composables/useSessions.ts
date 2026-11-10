import { computed, nextTick, reactive, ref, watch } from "vue";
import RFB from "@novnc/novnc";
import { api } from "../lib/api";
import { errorText } from "../lib/format";
import type { ConnectResponse, LiveSession, Machine } from "../types";
import { machineById } from "./useMachines";
import { settings } from "./useSettings";

/**
 * 多会话的核心约定：
 * 每台机器最多一个 RFB 对象，挂在始终留在 DOM 里的容器上。
 * `switchTo` 只改 `activeId`。样式把其他会话藏起来，连接不断。
 * 只有首次连接、手动重连、断线自动重连才会新建 RFB，此时 generation 加一。
 */
const sessions = reactive<Record<string, LiveSession>>({});
const activeId = ref<string | null>(null);
const rfbById = new Map<string, RFB>();
const tickets: Record<string, number> = {};
const reconnectTimers = new Map<string, number>();
const resolutionTimers = new Map<string, number>();
let e2e = false;

export function useSessions() {
  const sessionList = computed(() => Object.values(sessions));
  return {
    sessions,
    sessionList,
    activeId,
    connectMachine,
    disconnectMachine,
    reconnectMachine,
    switchTo,
    focusSession,
    applyLiveOptions,
    isLive,
    setE2eMode,
  };
}

export function isLive(id: string): boolean {
  const session = sessions[id];
  if (!session) return false;
  return session.status === "connected" || session.status === "connecting" || session.status === "reconnecting";
}

export function switchTo(id: string): void {
  activeId.value = id;
  queueMicrotask(() => focusSession(id));
}

export function focusSession(id: string): void {
  if (activeId.value !== id) return;
  const rfb = rfbById.get(id);
  try {
    rfb?.focus({ preventScroll: true });
  } catch {
    // 画布还没插入时忽略。
  }
}

export function setE2eMode(enabled: boolean): void {
  e2e = enabled;
  if (enabled) void publishE2e();
}

export async function connectMachine(
  machine: Machine,
  options?: { force?: boolean; keepActive?: boolean },
): Promise<void> {
  const existing = rfbById.get(machine.id);
  const current = sessions[machine.id];
  const alreadyUp =
    !options?.force &&
    existing &&
    current &&
    (current.status === "connected" || current.status === "connecting");
  if (alreadyUp) {
    if (!options?.keepActive) activeId.value = machine.id;
    focusSession(machine.id);
    return;
  }

  const myTicket = (tickets[machine.id] ?? 0) + 1;
  tickets[machine.id] = myTicket;
  if (!options?.keepActive) activeId.value = machine.id;

  if (existing) {
    try {
      existing.disconnect();
    } catch {
      // 旧客户端可能已经自己断了。
    }
    rfbById.delete(machine.id);
    stopResolutionWatch(machine.id);
  }
  clearReconnect(machine.id);

  const generation = (current?.generation ?? 0) + 1;
  sessions[machine.id] = {
    machineId: machine.id,
    status: options?.force && current ? "reconnecting" : "connecting",
    generation,
    connectedAt: null,
    resolution: current?.resolution ?? null,
    desktopName: current?.desktopName ?? "",
    error: null,
    transport: machine.transport,
    webUrl: null,
    manual: false,
    attempt: options?.force ? (current?.attempt ?? 0) : 0,
  };

  let response: ConnectResponse;
  try {
    response = await api.connectMachine(machine.id);
  } catch (error) {
    if (tickets[machine.id] !== myTicket) return;
    sessions[machine.id].status = "error";
    sessions[machine.id].error = errorText(error);
    scheduleReconnect(machine.id);
    return;
  }
  if (tickets[machine.id] !== myTicket) return;

  if (response.transport === "web") {
    if (!response.webUrl) {
      sessions[machine.id].status = "error";
      sessions[machine.id].error = "缺少网页地址";
      return;
    }
    sessions[machine.id].webUrl = response.webUrl;
    sessions[machine.id].status = "connected";
    sessions[machine.id].connectedAt = Date.now();
    return;
  }

  if (!response.wsUrl) {
    sessions[machine.id].status = "error";
    sessions[machine.id].error = "没有可用的 WebSocket 地址";
    scheduleReconnect(machine.id);
    return;
  }

  await nextTick();
  if (tickets[machine.id] !== myTicket) return;
  const host = document.getElementById(`vnc-host-${machine.id}`);
  if (!(host instanceof HTMLElement)) {
    sessions[machine.id].status = "error";
    sessions[machine.id].error = "画面容器还没有挂上";
    return;
  }
  host.replaceChildren();

  const rfb = new RFB(host, response.wsUrl, {
    credentials: response.password ? { password: response.password } : undefined,
    shared: true,
  });
  applyOne(rfb, machine);
  rfbById.set(machine.id, rfb);
  bindRfb(machine.id, generation, rfb, response.password);
  watchResolution(machine.id, host);
  if (activeId.value === machine.id) rfb.focus({ preventScroll: true });
}

export async function disconnectMachine(id: string): Promise<void> {
  const session = sessions[id];
  if (session) {
    session.manual = true;
    session.status = "disconnected";
    session.connectedAt = null;
    session.webUrl = null;
  }
  clearReconnect(id);
  const rfb = rfbById.get(id);
  if (rfb) {
    try {
      rfb.disconnect();
    } catch {
      // 忽略重复断开。
    }
    rfbById.delete(id);
  }
  stopResolutionWatch(id);
  const host = document.getElementById(`vnc-host-${id}`);
  host?.replaceChildren();
  try {
    await api.disconnectMachine(id);
  } catch (error) {
    if (session) {
      session.status = "error";
      session.error = errorText(error);
    }
  }
}

export function reconnectMachine(id: string): void {
  const machine = machineById(id);
  if (!machine) return;
  const session = sessions[id];
  if (session) session.manual = false;
  void connectMachine(machine, { force: true });
}

export function applyLiveOptions(): void {
  for (const [id, rfb] of rfbById) {
    const machine = machineById(id);
    if (machine) applyOne(rfb, machine);
  }
}

export function forgetSession(id: string): void {
  void disconnectMachine(id);
  delete sessions[id];
  if (activeId.value === id) activeId.value = null;
}

function applyOne(rfb: RFB, machine: Machine): void {
  const fit = settings.value.scaleMode === "fit";
  rfb.scaleViewport = fit;
  rfb.clipViewport = !fit;
  rfb.resizeSession = false;
  rfb.viewOnly = machine.viewOnly;
  rfb.showDotCursor = settings.value.showDotCursor;
  rfb.qualityLevel = clampQuality(settings.value.qualityLevel);
  rfb.compressionLevel = 2;
  rfb.background = "transparent";
}

function bindRfb(id: string, generation: number, rfb: RFB, password: string | null): void {
  const current = () => sessions[id]?.generation === generation;

  rfb.addEventListener("connect", () => {
    if (!current()) return;
    const session = sessions[id];
    session.status = "connected";
    session.connectedAt = Date.now();
    session.error = null;
    session.attempt = 0;
    readResolution(id);
    focusSession(id);
  });

  rfb.addEventListener("disconnect", () => {
    if (!current()) return;
    const session = sessions[id];
    if (!session || session.manual || session.status === "disconnected") return;
    session.connectedAt = null;
    scheduleReconnect(id);
  });

  rfb.addEventListener("credentialsrequired", () => {
    if (!current()) return;
    if (password) {
      rfb.sendCredentials({ password });
      return;
    }
    const session = sessions[id];
    session.manual = true;
    session.status = "error";
    session.error = "服务器要求密码。在机器设置里保存 VNC 密码后再连接。";
  });

  rfb.addEventListener("securityfailure", (event) => {
    if (!current()) return;
    const detail = event.detail as { reason?: string } | undefined;
    const session = sessions[id];
    session.manual = true;
    session.status = "error";
    session.error = detail?.reason ? `认证失败：${detail.reason}` : "认证失败，请检查 VNC 密码";
  });

  rfb.addEventListener("desktopname", (event) => {
    if (!current()) return;
    const detail = event.detail as { name?: string } | undefined;
    if (detail?.name) sessions[id].desktopName = detail.name;
  });
}

function scheduleReconnect(id: string): void {
  const session = sessions[id];
  if (!session || session.manual) return;
  if (!settings.value.autoReconnect) {
    session.status = "disconnected";
    return;
  }
  clearReconnect(id);
  session.status = "reconnecting";
  session.attempt += 1;
  const base = settings.value.reconnectIntervalMs;
  const delay = session.attempt > 8 ? Math.max(base, 8000) : base;
  const timer = window.setTimeout(() => {
    const machine = machineById(id);
    if (!machine || sessions[id]?.manual) return;
    void connectMachine(machine, { force: true, keepActive: true });
  }, delay);
  reconnectTimers.set(id, timer);
}

function clearReconnect(id: string): void {
  const timer = reconnectTimers.get(id);
  if (timer !== undefined) {
    window.clearTimeout(timer);
    reconnectTimers.delete(id);
  }
}

function watchResolution(id: string, host: HTMLElement): void {
  stopResolutionWatch(id);
  const timer = window.setInterval(() => readResolution(id, host), 1000);
  resolutionTimers.set(id, timer);
  readResolution(id, host);
}

function stopResolutionWatch(id: string): void {
  const timer = resolutionTimers.get(id);
  if (timer !== undefined) {
    window.clearInterval(timer);
    resolutionTimers.delete(id);
  }
}

function readResolution(id: string, host?: HTMLElement): void {
  const root = host ?? document.getElementById(`vnc-host-${id}`);
  const canvas = root?.querySelector("canvas");
  if (!canvas || canvas.width === 0 || canvas.height === 0) return;
  const session = sessions[id];
  if (!session) return;
  if (session.resolution?.width === canvas.width && session.resolution?.height === canvas.height) return;
  session.resolution = { width: canvas.width, height: canvas.height };
}

function clampQuality(value: number): number {
  if (!Number.isInteger(value)) return 6;
  return Math.min(9, Math.max(0, value));
}

async function publishE2e(): Promise<void> {
  if (!e2e) return;
  const payload = {
    activeId: activeId.value,
    sessions: Object.values(sessions).map((session) => ({
      id: session.machineId,
      status: session.status,
      generation: session.generation,
      resolution: session.resolution,
      error: session.error,
      attempt: session.attempt,
    })),
  };
  try {
    await api.reportE2e(payload);
  } catch {
    // 验证报告写失败不应该影响会话本身。
  }
}

watch(sessions, () => void publishE2e(), { deep: true });
watch(activeId, () => void publishE2e());
