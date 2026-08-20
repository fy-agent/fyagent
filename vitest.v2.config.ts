import { fileURLToPath } from "node:url";
import { configDefaults, defineConfig } from "vitest/config";

export default defineConfig({
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  test: {
    environment: "jsdom",
    environmentOptions: {
      jsdom: {
        url: "http://localhost/",
      },
    },
    setupFiles: ["./tests/setupGlobals.ts", "./tests/v2/app/setup.ts"],
    include: ["tests/v2/**/*.{test,spec}.{ts,tsx}"],
    exclude: [...configDefaults.exclude, "tests/v2-browser/**"],
    globals: true,
    clearMocks: true,
    restoreMocks: true,
  },
});
