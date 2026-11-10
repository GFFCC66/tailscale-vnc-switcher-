import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
// @ts-expect-error process 的类型由 Node 提供，这里不额外装 @types/node
import process from "node:process";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(() => ({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: "esnext",
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
}));
