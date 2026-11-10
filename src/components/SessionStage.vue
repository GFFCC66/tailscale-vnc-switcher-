<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { kindLabel, transportLabel } from "../lib/format";
import { machineById } from "../composables/useMachines";
import { reconnectMachine, useSessions } from "../composables/useSessions";

const emit = defineEmits<{ connect: [id: string]; edit: [id: string] }>();
const { sessionList, activeId } = useSessions();
const toast = ref("");
let toastTimer = 0;

const activeSession = computed(() => (activeId.value ? sessionList.value.find((item) => item.machineId === activeId.value) : undefined));
const activeMachine = computed(() => (activeId.value ? machineById(activeId.value) : undefined));

watch(activeId, (id) => {
  if (!id) return;
  const machine = machineById(id);
  if (!machine) return;
  toast.value = machine.name;
  window.clearTimeout(toastTimer);
  toastTimer = window.setTimeout(() => {
    toast.value = "";
  }, 700);
});

function retry(id: string) {
  reconnectMachine(id);
}
</script>

<template>
  <main class="stage">
    <div v-if="!activeSession" class="ready">
      <div class="card">
        <template v-if="activeMachine">
          <h3>{{ activeMachine.name }}</h3>
          <p>{{ activeMachine.notes || "还没有连上。连接会留在后台，之后切换只是把画面掀开。" }}</p>
          <p class="hint" style="margin-top: 8px">
            {{ kindLabel(activeMachine.kind) }} · {{ transportLabel(activeMachine.transport) }}
            <template v-if="activeMachine.hasPassword"> · 已保存密码</template>
          </p>
          <div class="actions">
            <button class="primary" type="button" @click="emit('connect', activeMachine.id)">连接</button>
            <button class="ghost" type="button" @click="emit('edit', activeMachine.id)">编辑</button>
          </div>
        </template>
        <template v-else>
          <h3>一台窗口，多台云电脑</h3>
          <p>从左侧选择机器。已经连上的桌面会一直保持，切换时不会重新握手。</p>
        </template>
      </div>
    </div>

    <div class="layers">
      <div
        v-for="session in sessionList"
        :key="session.machineId"
        class="layer"
        :class="{ active: session.machineId === activeId }"
        :data-session="session.machineId"
        :data-generation="session.generation"
        :data-status="session.status"
      >
        <div :id="`vnc-host-${session.machineId}`" class="viewport"></div>
        <iframe v-if="session.webUrl" class="viewport web" :src="session.webUrl" title="网页桌面"></iframe>
        <div v-if="session.status === 'connecting' || session.status === 'reconnecting'" class="overlay">
          <div class="card" style="text-align: center">
            <div class="spinner" style="margin-inline: auto"></div>
            <h3>{{ session.status === "reconnecting" ? "正在重连" : "正在连接" }}</h3>
            <p>
              {{ machineById(session.machineId)?.name }}
              <template v-if="session.attempt > 1"> · 第 {{ session.attempt }} 次</template>
            </p>
          </div>
        </div>
        <div v-else-if="session.status === 'error'" class="overlay">
          <div class="card">
            <h3>连不上</h3>
            <p>{{ session.error }}</p>
            <div class="actions">
              <button class="primary" type="button" @click="retry(session.machineId)">重试</button>
            </div>
          </div>
        </div>
        <div v-else-if="session.status === 'disconnected'" class="overlay">
          <div class="card">
            <h3>已断开</h3>
            <p>这台机器的会话已经关掉。其他机器如果还连着，不会受到影响。</p>
            <div class="actions">
              <button class="primary" type="button" @click="retry(session.machineId)">重新连接</button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <Transition name="toast">
      <div v-if="toast && activeSession" class="switch-toast">{{ toast }}</div>
    </Transition>
  </main>
</template>
