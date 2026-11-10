<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import { tauriAvailable } from "../lib/api";

async function minimize() {
  if (!tauriAvailable()) return;
  await getCurrentWindow().minimize();
}
async function maximize() {
  if (!tauriAvailable()) return;
  await getCurrentWindow().toggleMaximize();
}
async function close() {
  if (!tauriAvailable()) return;
  await getCurrentWindow().close();
}
</script>

<template>
  <header class="titlebar" data-tauri-drag-region>
    <div class="brand" data-tauri-drag-region>
      <svg class="mark" viewBox="0 0 18 18" aria-hidden="true">
        <rect x="1" y="3" width="11" height="8" rx="1.5" fill="none" stroke="currentColor" stroke-width="1.4" opacity="0.45" />
        <rect x="5" y="6" width="12" height="9" rx="1.6" fill="#7ee0c6" />
      </svg>
      <div data-tauri-drag-region>
        <h1 data-tauri-drag-region>云桌面切换</h1>
      </div>
      <p data-tauri-drag-region>Tailscale</p>
    </div>
    <div class="title-spacer" data-tauri-drag-region></div>
    <div class="window-controls">
      <button class="win-btn" type="button" aria-label="最小化" @click="minimize">
        <svg width="10" height="10" viewBox="0 0 10 10"><path d="M1 5h8" stroke="currentColor" stroke-width="1.2" /></svg>
      </button>
      <button class="win-btn" type="button" aria-label="最大化" @click="maximize">
        <svg width="10" height="10" viewBox="0 0 10 10"><rect x="1.5" y="1.5" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1.2" /></svg>
      </button>
      <button class="win-btn close" type="button" aria-label="关闭" @click="close">
        <svg width="10" height="10" viewBox="0 0 10 10"><path d="M2 2l6 6M8 2L2 8" stroke="currentColor" stroke-width="1.2" /></svg>
      </button>
    </div>
  </header>
</template>
