import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { reactive, ref } from "vue";
import { api, tauriAvailable } from "../lib/api";
import { errorText } from "../lib/format";
import type { Machine, MachineDraft, ProbeEvent } from "../types";

const machines = ref<Machine[]>([]);
const probes = reactive<Record<string, ProbeEvent>>({});
const loaded = ref(false);
const loadError = ref<string | null>(null);

let unlisten: UnlistenFn | null = null;

export function useMachines() {
  return { machines, probes, loaded, loadError, reload, saveMachine, removeMachine, machineById };
}

export function machineById(id: string): Machine | undefined {
  return machines.value.find((machine) => machine.id === id);
}

export async function reload(): Promise<void> {
  if (!tauriAvailable()) {
    loadError.value = "请在桌面窗口里运行。开发时用 npm run tauri dev，浏览器单独打开没有本机 VNC 代理。";
    loaded.value = true;
    return;
  }
  try {
    machines.value = await api.listMachines();
    loadError.value = null;
    void refreshProbes();
  } catch (error) {
    loadError.value = errorText(error);
  } finally {
    loaded.value = true;
  }
}

export async function saveMachine(draft: MachineDraft): Promise<Machine> {
  const saved = await api.upsertMachine(draft);
  const index = machines.value.findIndex((machine) => machine.id === saved.id);
  if (index >= 0) machines.value.splice(index, 1, saved);
  else machines.value.push(saved);
  void refreshProbes();
  return saved;
}

export async function removeMachine(id: string): Promise<void> {
  await api.deleteMachine(id);
  machines.value = machines.value.filter((machine) => machine.id !== id);
  delete probes[id];
}

export async function bindProbeEvents(): Promise<void> {
  if (!tauriAvailable() || unlisten) return;
  unlisten = await listen<ProbeEvent>("machine-probe", (event) => {
    probes[event.payload.id] = event.payload;
  });
}

async function refreshProbes(): Promise<void> {
  try {
    const results = await api.probeMachines();
    for (const probe of results) probes[probe.id] = probe;
  } catch {
    // 探测失败时保留上一次的状态，侧边栏仍显示「检测中」。
  }
}
