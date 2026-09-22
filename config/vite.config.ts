import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { codeInspectorPlugin } from "code-inspector-plugin";

const sourceRoot = fileURLToPath(new URL("../src", import.meta.url));

export default defineConfig(({ command }) => {
  return {
    root: sourceRoot,
    envDir: sourceRoot,
    css: { postcss: fileURLToPath(new URL(".", import.meta.url)) },
    plugins: [
      command === "serve" &&
        codeInspectorPlugin({
          bundler: "vite",
        }),
      react(),
    ].filter(Boolean),
    base: "./",
    build: {
      outDir: "../dist",
      emptyOutDir: true,
      manifest: true,
      cssCodeSplit: true,
      rollupOptions: {
        output: {
          // Keep each named entry's dependency closure together.
          manualChunks: {
            "vendor-react": ["react", "react-dom", "react-router-dom"],
            "vendor-query": ["@tanstack/react-query"],
            "vendor-motion": ["framer-motion"],
            // Route-only dialog, popover, select and tabs follow their lazy
            // consumers instead of joining the shell's tooltip/collapse group.
            "vendor-radix": [
              "@radix-ui/react-checkbox",
              "@radix-ui/react-switch",
              "@radix-ui/react-tooltip",
              "@radix-ui/react-collapsible",
            ],
            "vendor-tauri": ["@tauri-apps/api"],
          },
        },
      },
    },
    server: {
      port: 3000,
      strictPort: true,
    },
    resolve: {
      alias: {
        "@": path.resolve(sourceRoot),
      },
    },
    clearScreen: false,
    envPrefix: ["VITE_", "TAURI_"],
  };
});
