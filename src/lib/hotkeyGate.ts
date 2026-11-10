import { ref } from "vue";

/** 对话框打开时暂停全局快捷键，避免输入框里的 Ctrl+1 把会话切走。 */
export const hotkeysSuspended = ref(false);
