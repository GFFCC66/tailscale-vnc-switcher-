import { ref } from "vue";
import { api, tauriAvailable } from "../lib/api";
import { defaultSettings, type Settings } from "../types";

export const settings = ref<Settings>(defaultSettings());
export const configPath = ref("");

export function useSettings() {
  return { settings, configPath, loadSettings, saveSettings, applyTheme };
}

export async function loadSettings(): Promise<void> {
  if (!tauriAvailable()) return;
  settings.value = await api.getSettings();
  applyTheme(settings.value.theme);
  try {
    configPath.value = await api.configPath();
  } catch {
    configPath.value = "";
  }
}

export async function saveSettings(next: Settings): Promise<Settings> {
  const saved = await api.saveSettings(next);
  settings.value = saved;
  applyTheme(saved.theme);
  return saved;
}

export function applyTheme(theme: Settings["theme"]): void {
  document.documentElement.dataset.theme = theme;
}
