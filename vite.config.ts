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
          // react-markdown 及其 unified/remark/rehype/micromark 生态单独成 chunk，
          // 避免被 vendor 兜底吞掉、随首屏一起加载。
          // 只归集 markdown 专用的大库；通用小工具库仍留在 vendor，避免循环 chunk。
          const markdownPrefixes = [
            "react-markdown", "unified", "remark-", "rehype-", "mdast-", "hast-",
            "unist-", "micromark", "vfile", "devlop",
          ];
          if (markdownPrefixes.some((p) => id.includes(`node_modules/${p}`))) return "markdown";
          return "vendor";
        },
      },
    },
  },
}));
