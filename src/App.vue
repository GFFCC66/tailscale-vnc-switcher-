<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import TitleBar from "./components/TitleBar.vue";
import Sidebar from "./components/Sidebar.vue";
import SessionStage from "./components/SessionStage.vue";
import StatusBar from "./components/StatusBar.vue";
import MachineDialog from "./components/MachineDialog.vue";
import SettingsDialog from "./components/SettingsDialog.vue";
import { api, tauriAvailable } from "./lib/api";
import { isTypingTarget, matchCombo, matchJump, parseCombo } from "./lib/hotkeys";
import { hotkeysSuspended } from "./lib/hotkeyGate";
import { cycleTargets, flattenGroups, groupMachines, jumpIndex } from "./lib/switchOrder";
import type { Machine } from "./types";
import { bindProbeEvents, machineById, reload, removeMachine, useMachines } from "./composables/useMachines";
import { loadSettings, useSettings } from "./composables/useSettings";
import {
  applyLiveOptions,
  connectMachine,
  disconnectMachine,
  forgetSession,
  isLive,
  reconnectMachine,
  setE2eMode,
  switchTo,
  useSessions,
} from "./composables/useSessions";

const { machines } = useMachines();
const { settings } = useSettings();
const { activeId } = useSessions();

const editing = ref<Machine | null>(null);
const showMachine = ref(false);
const showSettings = ref(false);
const fullscreen = ref(false);

const ordered = computed(() => flattenGroups(groupMachines(machines.value, "")));

watch(
  () => [settings.value.scaleMode, settings.value.qualityLevel, settings.value.showDotCursor, machines.value] as const,
  () => applyLiveOptions(),
);

function openCreate() {
  editing.value = null;
  showMachine.value = true;
}

function openEdit(machine: Machine) {
  editing.value = machine;
  showMachine.value = true;
}

function onSelect(machine: Machine) {
  if (isLive(machine.id)) switchTo(machine.id);
  else void connectMachine(machine);
}

async function onDelete(id: string) {
  forgetSession(id);
  await removeMachine(id);
}

function cycle(direction: 1 | -1) {
  const targets = ordered.value.map((machine) => ({ id: machine.id, live: isLive(machine.id) }));
  const next = cycleTargets(targets, activeId.value, direction, targets.some((item) => item.live));
  if (!next) return;
  if (isLive(next)) switchTo(next);
  else activeId.value = next;
}

async function toggleFullscreen() {
  if (!tauriAvailable()) {
    fullscreen.value = !fullscreen.value;
    return;
  }
  const current = getCurrentWindow();
  const next = !(await current.isFullscreen());
  await current.setFullscreen(next);
  fullscreen.value = next;
}

function onKey(event: KeyboardEvent) {
  if (hotkeysSuspended.value || isTypingTarget(event.target)) return;
  if (event.key === "Escape" && fullscreen.value) {
    event.preventDefault();
    void toggleFullscreen();
    return;
  }
  const jump = matchJump(event, settings.value.hotkeys.jumpModifier);
  if (jump) {
    event.preventDefault();
    event.stopPropagation();
    const index = jumpIndex(jump, ordered.value.length);
    if (index === null) return;
    onSelect(ordered.value[index]);
    return;
  }
  const next = parseCombo(settings.value.hotkeys.next);
  const previous = parseCombo(settings.value.hotkeys.previous);
  const full = parseCombo(settings.value.hotkeys.fullscreen);
  const again = parseCombo(settings.value.hotkeys.reconnect);
  if (next && matchCombo(event, next)) {
    event.preventDefault();
    event.stopPropagation();
    cycle(1);
    return;
  }
  if (previous && matchCombo(event, previous)) {
    event.preventDefault();
    event.stopPropagation();
    cycle(-1);
    return;
  }
  if (full && matchCombo(event, full)) {
    event.preventDefault();
    event.stopPropagation();
    void toggleFullscreen();
    return;
  }
  if (again && matchCombo(event, again) && activeId.value) {
    event.preventDefault();
    event.stopPropagation();
    reconnectMachine(activeId.value);
  }
}

onMounted(async () => {
  window.addEventListener("keydown", onKey, true);
  await loadSettings();
  await reload();
  await bindProbeEvents();
  if (tauriAvailable() && (await api.e2eEnabled())) {
    setE2eMode(true);
    const locals = machines.value.filter((machine) => machine.host === "127.0.0.1");
    await Promise.all(locals.map((machine) => connectMachine(machine, { keepActive: true })));
    if (locals[0]) switchTo(locals[0].id);
  } else if (!activeId.value && ordered.value[0]) {
    activeId.value = ordered.value[0].id;
  }
});

onUnmounted(() => window.removeEventListener("keydown", onKey, true));

function reconnectCurrent() {
  if (activeId.value) reconnectMachine(activeId.value);
}
async function disconnectCurrent() {
  if (activeId.value) await disconnectMachine(activeId.value);
}
function connectById(id: string) {
  const machine = machineById(id);
  if (machine) void connectMachine(machine);
}
function editById(id: string) {
  const machine = machineById(id);
  if (machine) openEdit(machine);
}
</script>

<template>
  <div class="shell" :class="{ fullscreen }" :data-theme="settings.theme">
    <TitleBar />
    <div class="body">
      <Sidebar
        @add="openCreate"
        @edit="openEdit"
        @settings="showSettings = true"
        @select="onSelect"
        @disconnect="disconnectMachine"
      />
      <SessionStage @connect="connectById" @edit="editById" />
    </div>
    <StatusBar @fullscreen="toggleFullscreen" @reconnect="reconnectCurrent" @disconnect="disconnectCurrent" />
    <button v-if="fullscreen" class="text-btn exit-fullscreen" type="button" @click="toggleFullscreen">退出全屏</button>
    <MachineDialog
      v-if="showMachine"
      :machine="editing"
      @close="showMachine = false"
      @deleted="onDelete"
    />
    <SettingsDialog v-if="showSettings" @close="showSettings = false" />
  </div>
</template>
