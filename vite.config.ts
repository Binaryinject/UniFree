import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      // 排除 Rust 构建目录，避免文件锁定问题
      ignored: ["**/src-tauri/target/**"],
    },
  },
  build: {
    rollupOptions: {
      output: {
        // 将 node_modules 按库拆分，配合 Tab 懒加载减小首屏体积
        manualChunks(id: string) {
          if (!id.includes("node_modules")) return;
          if (id.includes("node_modules/@mui/") || id.includes("node_modules/@emotion/")) return "mui";
          if (id.includes("node_modules/@phosphor-icons/")) return "icons";
          if (id.includes("node_modules/i18next/") || id.includes("node_modules/react-i18next/")) return "i18n";
          if (id.includes("node_modules/@tauri-apps/")) return "tauri";
          if (
            id.includes("node_modules/react/") ||
            id.includes("node_modules/react-dom/") ||
            id.includes("node_modules/scheduler/")
          ) {
            return "react";
          }
          return "vendor";
        },
      },
    },
  },
}));
