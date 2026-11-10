<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { connectionLabel, formatDuration } from "../lib/format";
import { qualityFromLatency } from "../lib/quality";
import { machineById, useMachines } from "../composables/useMachines";
import { useSessions } from "../composables/useSessions";

const emit = defineEmits<{ fullscreen: []; reconnect: []; disconnect: [] }>();
const { probes } = useMachines();
const { activeId, sessions } = useSessions();
const now = ref(Date.now());
let timer = 0;

onMounted(() => {
  timer = window.setInterval(() => {
    now.value = Date.now();
  }, 1000);
});
onUnmounted(() => window.clearInterval(timer));

const machine = computed(() => (activeId.value ? machineById(activeId.value) : undefined));
const session = computed(() => (activeId.value ? sessions[activeId.value] : undefined));
const probe = computed(() => (activeId.value ? probes[activeId.value] : undefined));
const quality = computed(() =>
  qualityFromLatency(probe.value ? probe.value.online : null, probe.value?.latencyMs ?? null),
);
const duration = computed(() => {
  if (!session.value?.connectedAt) return "—";
  return formatDuration(now.value - session.value.connectedAt);
});
const resolution = computed(() => {
  const size = session.value?.resolution;
  if (!size) return "—";
  return `${size.width}×${size.height}`;
});
</script>

<template>
  <footer class="statusbar">
    <span class="status-name">{{ machine?.name ?? "未选择" }}</span>
    <span>{{ connectionLabel(session?.status) }}</span>
    <span class="mono">{{ resolution }}</span>
    <span class="mono" :title="session?.desktopName || undefined">{{ duration }}</span>
    <span v-if="probe?.online && probe.latencyMs !== null">{{ probe.latencyMs }} ms · {{ quality.label }}</span>
    <span v-else>{{ quality.label }}</span>
    <span class="status-spacer"></span>
    <span class="status-actions">
      <button class="text-btn" type="button" :disabled="!machine" @click="emit('reconnect')">重连</button>
      <button class="text-btn" type="button" :disabled="!session || session.status === 'disconnected'" @click="emit('disconnect')">断开</button>
      <button class="text-btn" type="button" @click="emit('fullscreen')">全屏</button>
    </span>
  </footer>
</template>
