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
          // Let Rollup keep each named entry's dependency closure together.
          // A path-based catch-all split React's helpers from React and put
          // hook consumers in the reverse dependency, breaking production.
          manualChunks: {
            "vendor-react": ["react", "react-dom", "react-router-dom"],
            "vendor-query": ["@tanstack/react-query"],
            "vendor-motion": ["framer-motion"],
            "vendor-radix": [
              "@radix-ui/react-dialog",
              "@radix-ui/react-checkbox",
              "@radix-ui/react-popover",
              "@radix-ui/react-select",
              "@radix-ui/react-switch",
              "@radix-ui/react-tabs",
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
