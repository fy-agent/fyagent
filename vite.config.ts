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
        "@": path.resolve(__dirname, "./src"),
      },
    },
    clearScreen: false,
    envPrefix: ["VITE_", "TAURI_"],
  };
});
