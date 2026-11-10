<script setup lang="ts">
import { computed, onBeforeUnmount, reactive, ref, watch } from "vue";
import { errorText } from "../lib/format";
import { hotkeysSuspended } from "../lib/hotkeyGate";
import type { Machine, MachineDraft, MachineKind, Transport } from "../types";
import { saveMachine } from "../composables/useMachines";

const props = defineProps<{ machine: Machine | null }>();
const emit = defineEmits<{ close: []; saved: []; deleted: [id: string] }>();

const form = reactive({
  name: "",
  host: "",
  port: 5900,
  transport: "vnc-tcp" as Transport,
  kind: "x11vnc" as MachineKind,
  group: "",
  tags: "",
  notes: "",
  directUrl: "",
  viewOnly: false,
  password: "",
  clearPassword: false,
});
const error = ref("");
const confirmDelete = ref(false);
const saving = ref(false);

const title = computed(() => (props.machine ? "编辑机器" : "添加机器"));
const needsUrl = computed(() => form.transport === "websocket" || form.transport === "web");

watch(
  () => props.machine,
  (machine) => {
    form.name = machine?.name ?? "";
    form.host = machine?.host ?? "";
    form.port = machine?.port ?? 5900;
    form.transport = machine?.transport ?? "vnc-tcp";
    form.kind = machine?.kind ?? "x11vnc";
    form.group = machine?.group ?? "";
    form.tags = machine?.tags.join(", ") ?? "";
    form.notes = machine?.notes ?? "";
    form.directUrl = machine?.directUrl ?? "";
    form.viewOnly = machine?.viewOnly ?? false;
    form.password = "";
    form.clearPassword = false;
    error.value = "";
    confirmDelete.value = false;
  },
  { immediate: true },
);

hotkeysSuspended.value = true;
onBeforeUnmount(() => {
  hotkeysSuspended.value = false;
});

function close() {
  emit("close");
}

async function submit() {
  error.value = "";
  saving.value = true;
  const draft: MachineDraft = {
    id: props.machine?.id,
    name: form.name.trim(),
    host: form.host.trim(),
    port: Number(form.port),
    transport: form.transport,
    kind: form.kind,
    group: form.group.trim(),
    tags: form.tags
      .split(/[,，]/)
      .map((tag) => tag.trim())
      .filter(Boolean),
    notes: form.notes.trim(),
    directUrl: form.directUrl.trim() || null,
    viewOnly: form.viewOnly,
  };
  if (form.clearPassword) draft.password = "";
  else if (form.password) draft.password = form.password;
  try {
    await saveMachine(draft);
    emit("saved");
    close();
  } catch (cause) {
    error.value = errorText(cause);
  } finally {
    saving.value = false;
  }
}

function remove() {
  if (!props.machine) return;
  if (!confirmDelete.value) {
    confirmDelete.value = true;
    return;
  }
  emit("deleted", props.machine.id);
  close();
}
</script>

<template>
  <div class="modal-root" role="presentation">
    <div class="dialog" role="dialog" aria-modal="true" :aria-label="title">
      <header>
        <h3>{{ title }}</h3>
        <button class="icon-btn" type="button" aria-label="关闭" @click="close">×</button>
      </header>
      <form @submit.prevent="submit">
        <div class="grid-2">
          <label class="field">名称
            <input v-model="form.name" required maxlength="64" autofocus />
          </label>
          <label class="field">分组
            <input v-model="form.group" maxlength="32" placeholder="例如 云电脑" />
          </label>
        </div>
        <div class="grid-2">
          <label class="field">Tailscale 地址
            <input v-model="form.host" required placeholder="100.x.x.x 或主机名" spellcheck="false" />
          </label>
          <label class="field">端口
            <input v-model.number="form.port" type="number" min="1" max="65535" required />
          </label>
        </div>
        <div class="grid-2">
          <label class="field">类型
            <select v-model="form.kind">
              <option value="x11vnc">x11vnc</option>
              <option value="tigervnc">TigerVNC</option>
              <option value="stream">原生 stream</option>
              <option value="web-live">网页 live view</option>
            </select>
          </label>
          <label class="field">传输
            <select v-model="form.transport">
              <option value="vnc-tcp">VNC（TCP，经本机代理）</option>
              <option value="websocket">已有 WebSocket</option>
              <option value="web">网页</option>
            </select>
          </label>
        </div>
        <label v-if="needsUrl" class="field">直连地址
          <input v-model="form.directUrl" :placeholder="form.transport === 'web' ? 'https://...' : 'ws://...'" spellcheck="false" />
        </label>
        <label class="field">标签
          <input v-model="form.tags" placeholder="用逗号分开，例如 tailscale, work" />
        </label>
        <label class="field">备注
          <textarea v-model="form.notes" maxlength="500"></textarea>
        </label>
        <label class="field">VNC 密码
          <input v-model="form.password" type="password" autocomplete="new-password" :placeholder="machine?.hasPassword ? '已加密保存，留空则不修改' : '可选，只存在本机'" />
        </label>
        <label v-if="machine?.hasPassword" class="check">
          <input v-model="form.clearPassword" type="checkbox" />
          清除已保存的密码
        </label>
        <label class="check">
          <input v-model="form.viewOnly" type="checkbox" />
          只看，不把键盘鼠标送过去
        </label>
        <p class="hint">密码用 AES-256-GCM 加密后写进配置目录，列表里不会出现明文。</p>
        <p v-if="error" class="form-error">{{ error }}</p>
        <div class="actions">
          <button v-if="machine" class="danger push" type="button" @click="remove">
            {{ confirmDelete ? "确认删除" : "删除" }}
          </button>
          <button class="ghost" type="button" @click="close">取消</button>
          <button class="primary" type="submit" :disabled="saving">{{ saving ? "保存中" : "保存" }}</button>
        </div>
      </form>
    </div>
  </div>
</template>
