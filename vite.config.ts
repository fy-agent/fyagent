import path from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { codeInspectorPlugin } from "code-inspector-plugin";

export default defineConfig(({ command, mode }) => {
  const standalonePreview = mode === "standalone-preview";

  return {
    root: "src",
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
      cssCodeSplit: !standalonePreview,
      rollupOptions: {
        output: standalonePreview
          ? {
              inlineDynamicImports: true,
            }
          : {
              manualChunks(id) {
                if (!id.includes("node_modules")) return undefined;
                if (
                  id.includes("/react/") ||
                  id.includes("/react-dom/") ||
                  id.includes("/react-router") ||
                  id.includes("/scheduler/")
                ) {
                  return "vendor-react";
                }
                if (id.includes("/@tanstack/")) return "vendor-query";
                if (id.includes("/framer-motion/")) return "vendor-motion";
                if (id.includes("/@radix-ui/")) return "vendor-radix";
                if (id.includes("/@tauri-apps/")) return "vendor-tauri";
                return "vendor-shared";
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
        "@": path.resolve(__dirname, "./src"),
      },
    },
    clearScreen: false,
    envPrefix: ["VITE_", "TAURI_"],
  };
});
