<script setup lang="ts">
import { computed, ref } from "vue";
import { connectionLabel, formatEndpoint, kindLabel } from "../lib/format";
import { qualityFromLatency } from "../lib/quality";
import { flattenGroups, groupMachines } from "../lib/switchOrder";
import type { Machine } from "../types";
import { useMachines } from "../composables/useMachines";
import { useSessions } from "../composables/useSessions";

const emit = defineEmits<{
  add: [];
  edit: [machine: Machine];
  settings: [];
  select: [machine: Machine];
  disconnect: [id: string];
}>();

const query = ref("");
const { machines, probes, loaded, loadError } = useMachines();
const { sessions, activeId } = useSessions();

const groups = computed(() => groupMachines(machines.value, query.value));
const order = computed(() => flattenGroups(groupMachines(machines.value, "")));

function badge(id: string): number | null {
  const index = order.value.findIndex((machine) => machine.id === id);
  if (index < 0 || index > 8) return null;
  return index + 1;
}

function presence(machine: Machine): string {
  const probe = probes[machine.id];
  const quality = qualityFromLatency(probe ? probe.online : null, probe?.latencyMs ?? null);
  const link = connectionLabel(sessions[machine.id]?.status);
  const online = quality.level === "unknown" ? "检测中" : quality.level === "offline" ? "离线" : "在线";
  return `${online} · ${link}`;
}

function latency(machine: Machine): string {
  const probe = probes[machine.id];
  if (!probe?.online || probe.latencyMs === null) return "";
  const quality = qualityFromLatency(true, probe.latencyMs);
  return `${probe.latencyMs} ms · ${quality.label}`;
}

function dotClass(machine: Machine): string {
  const session = sessions[machine.id];
  if (session?.status === "connected") return "connected";
  if (session?.status === "connecting" || session?.status === "reconnecting") return session.status;
  if (session?.status === "error") return "error";
  const probe = probes[machine.id];
  return qualityFromLatency(probe ? probe.online : null, probe?.latencyMs ?? null).level;
}
</script>

<template>
  <aside class="sidebar">
    <div class="side-head">
      <h2>机器</h2>
      <span class="count">{{ machines.length }}</span>
      <div class="side-actions">
        <button class="icon-btn" type="button" aria-label="设置" title="设置" @click="emit('settings')">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none" stroke="currentColor" stroke-width="1.3">
            <circle cx="7" cy="7" r="2" />
            <path d="M7 1.2v1.6M7 11.2v1.6M1.2 7h1.6M11.2 7h1.6M2.8 2.8l1.1 1.1M10.1 10.1l1.1 1.1M11.2 2.8l-1.1 1.1M3.9 10.1l-1.1 1.1" />
          </svg>
        </button>
        <button class="icon-btn" type="button" aria-label="添加机器" title="添加机器" @click="emit('add')">
          <svg width="14" height="14" viewBox="0 0 14 14"><path d="M7 2v10M2 7h10" stroke="currentColor" stroke-width="1.4" /></svg>
        </button>
      </div>
    </div>
    <div class="search-wrap">
      <input v-model="query" class="search" type="search" placeholder="搜索名称、地址或标签" />
    </div>
    <div v-if="loadError" class="banner">{{ loadError }}</div>
    <div v-else-if="loaded && machines.length === 0" class="empty">还没有机器。点右上角加上第一台。</div>
    <div class="machine-scroll">
      <section v-for="group in groups" :key="group.name" class="group">
        <h3 class="group-name">{{ group.name }}</h3>
        <div
          v-for="machine in group.items"
          :key="machine.id"
          class="machine"
          :class="{ active: machine.id === activeId }"
          role="button"
          tabindex="0"
          @click="emit('select', machine)"
          @keydown.enter.prevent="emit('select', machine)"
        >
          <span class="index">{{ badge(machine.id) ?? "" }}</span>
          <span>
            <span class="machine-name">
              <span class="dot" :class="dotClass(machine)"></span>
              <span>{{ machine.name }}</span>
              <span class="kind">{{ kindLabel(machine.kind) }}</span>
            </span>
            <span class="endpoint">{{ formatEndpoint(machine.host, machine.port) }}</span>
            <span class="presence">{{ presence(machine) }}</span>
            <span v-if="machine.tags.length" class="tags">
              <span v-for="tag in machine.tags" :key="tag" class="tag">{{ tag }}</span>
            </span>
          </span>
          <span class="machine-side">
            <span class="latency">{{ latency(machine) }}</span>
            <span class="row-actions">
              <button class="icon-btn" type="button" aria-label="编辑" @click.stop="emit('edit', machine)">
                <svg width="12" height="12" viewBox="0 0 12 12"><path d="M2 8.5V10h1.5l5-5L7 3.5l-5 5z" fill="currentColor" /></svg>
              </button>
              <button
                v-if="sessions[machine.id] && sessions[machine.id].status !== 'disconnected'"
                class="icon-btn"
                type="button"
                aria-label="断开"
                @click.stop="emit('disconnect', machine.id)"
              >
                <svg width="12" height="12" viewBox="0 0 12 12"><path d="M3 3l6 6M9 3L3 9" stroke="currentColor" stroke-width="1.4" /></svg>
              </button>
            </span>
          </span>
        </div>
      </section>
    </div>
    <footer class="side-foot">
      <p><kbd>Ctrl</kbd>+<kbd>1</kbd>…<kbd>9</kbd> 跳转并连接</p>
      <p><kbd>Ctrl</kbd>+<kbd>Tab</kbd> 在已打开的会话之间切换</p>
    </footer>
  </aside>
</template>
