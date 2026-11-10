import { invoke } from "@tauri-apps/api/core";
import type { ConnectResponse, Machine, MachineDraft, ProbeEvent, Settings } from "../types";

export function tauriAvailable(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export const api = {
  listMachines: () => invoke<Machine[]>("list_machines"),
  upsertMachine: (input: MachineDraft) => invoke<Machine>("upsert_machine", { input }),
  deleteMachine: (id: string) => invoke<void>("delete_machine", { id }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  configPath: () => invoke<string>("config_path"),
  connectMachine: (id: string) => invoke<ConnectResponse>("connect_machine", { id }),
  disconnectMachine: (id: string) => invoke<void>("disconnect_machine", { id }),
  sessionTcpCount: (id: string) => invoke<number>("session_tcp_count", { id }),
  probeMachines: () => invoke<ProbeEvent[]>("probe_machines"),
  e2eEnabled: () => invoke<boolean>("e2e_enabled"),
  reportE2e: (payload: unknown) => invoke<void>("report_e2e", { payload }),
};
