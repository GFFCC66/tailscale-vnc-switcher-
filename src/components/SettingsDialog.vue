<script setup lang="ts">
import { onBeforeUnmount, reactive, ref, watch } from "vue";
import { comboFromEvent } from "../lib/hotkeys";
import { errorText } from "../lib/format";
import { hotkeysSuspended } from "../lib/hotkeyGate";
import type { Hotkeys, Settings } from "../types";
import { applyTheme, saveSettings, useSettings } from "../composables/useSettings";

const emit = defineEmits<{ close: [] }>();
const { settings, configPath } = useSettings();
const draft = reactive<Settings>(structuredClone(settings.value));
const error = ref("");
const saving = ref(false);
const capturing = ref<keyof Hotkeys | null>(null);

hotkeysSuspended.value = true;
watch(
  () => draft.theme,
  (theme) => applyTheme(theme),
);
onBeforeUnmount(() => {
  hotkeysSuspended.value = false;
  applyTheme(settings.value.theme);
});

function close() {
  emit("close");
}

function onCapture(event: KeyboardEvent) {
  if (!capturing.value) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.key === "Escape") {
    capturing.value = null;
    return;
  }
  const combo = comboFromEvent(event);
  if (!combo) return;
  if (capturing.value === "jumpModifier") return;
  draft.hotkeys[capturing.value] = combo;
  capturing.value = null;
}

async function submit() {
  error.value = "";
  if (draft.hotkeys.next === draft.hotkeys.previous) {
    error.value = "上一台和下一台的快捷键不能相同";
    return;
  }
  saving.value = true;
  try {
    await saveSettings(structuredClone(draft));
    close();
  } catch (cause) {
    error.value = errorText(cause);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="modal-root" @keydown="onCapture">
    <div class="dialog" role="dialog" aria-modal="true" aria-label="设置">
      <header>
        <h3>设置</h3>
        <button class="icon-btn" type="button" aria-label="关闭" @click="close">×</button>
      </header>
      <form @submit.prevent="submit">
        <label class="field">主题
          <div class="segmented">
            <button type="button" :aria-pressed="draft.theme === 'dark'" @click="draft.theme = 'dark'">暗色</button>
            <button type="button" :aria-pressed="draft.theme === 'light'" @click="draft.theme = 'light'">浅色</button>
          </div>
        </label>
        <label class="check">
          <input v-model="draft.autostart" type="checkbox" />
          开机自启
        </label>
        <label class="check">
          <input v-model="draft.autoReconnect" type="checkbox" />
          断线后自动重连
        </label>
        <div class="grid-2">
          <label class="field">重连间隔（毫秒）
            <input v-model.number="draft.reconnectIntervalMs" type="number" min="1000" max="60000" step="500" />
          </label>
          <label class="field">画面
            <select v-model="draft.scaleMode">
              <option value="fit">自适应缩放</option>
              <option value="native">原始分辨率</option>
            </select>
          </label>
        </div>
        <label class="field">画质 {{ draft.qualityLevel }}
          <input v-model.number="draft.qualityLevel" type="range" min="0" max="9" step="1" />
        </label>
        <label class="check">
          <input v-model="draft.showDotCursor" type="checkbox" />
          远程没提供光标时显示一个点
        </label>
        <div>
          <p class="hint">点下面的按键，再按下想用的组合。Esc 取消录制。</p>
          <div class="hotkey">
            <span>下一台</span>
            <button type="button" :class="{ capturing: capturing === 'next' }" @click="capturing = 'next'">
              {{ capturing === "next" ? "请按键…" : draft.hotkeys.next }}
            </button>
          </div>
          <div class="hotkey">
            <span>上一台</span>
            <button type="button" :class="{ capturing: capturing === 'previous' }" @click="capturing = 'previous'">
              {{ capturing === "previous" ? "请按键…" : draft.hotkeys.previous }}
            </button>
          </div>
          <div class="hotkey">
            <span>全屏</span>
            <button type="button" :class="{ capturing: capturing === 'fullscreen' }" @click="capturing = 'fullscreen'">
              {{ capturing === "fullscreen" ? "请按键…" : draft.hotkeys.fullscreen }}
            </button>
          </div>
          <div class="hotkey">
            <span>重连当前</span>
            <button type="button" :class="{ capturing: capturing === 'reconnect' }" @click="capturing = 'reconnect'">
              {{ capturing === "reconnect" ? "请按键…" : draft.hotkeys.reconnect }}
            </button>
          </div>
          <label class="field" style="margin-top: 10px">数字跳转的修饰键
            <select v-model="draft.hotkeys.jumpModifier">
              <option value="Ctrl">Ctrl + 1…9</option>
              <option value="Alt">Alt + 1…9</option>
              <option value="Ctrl+Alt">Ctrl + Alt + 1…9</option>
              <option value="Ctrl+Shift">Ctrl + Shift + 1…9</option>
            </select>
          </label>
        </div>
        <p v-if="configPath" class="hint">配置文件：{{ configPath }}</p>
        <p v-if="error" class="form-error">{{ error }}</p>
        <div class="actions">
          <button class="ghost" type="button" @click="close">取消</button>
          <button class="primary" type="submit" :disabled="saving">{{ saving ? "保存中" : "保存" }}</button>
        </div>
      </form>
    </div>
  </div>
</template>
