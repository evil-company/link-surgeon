import { defineConfig } from "vitest/config";
import { playwright } from "@vitest/browser-playwright";
import { sveltekit } from "@sveltejs/kit/vite";
// @ts-expect-error type error without @types/node package
import process from "node:process";
import type { UserConfig } from "vite";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig((): UserConfig => ({
    plugins: [sveltekit()],

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent Vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
        port: 1420,
        strictPort: true,
        host: host || "127.0.0.1",
        hmr: host
            ? {
                  protocol: "ws",
                  host,
                  port: 1421
              }
            : undefined,
        watch: {
            // 3. tell Vite to ignore watching `src-tauri`
            ignored: ["**/src-tauri/**"]
        }
    },
    test: {
        expect: { requireAssertions: true },
        projects: [
            {
                extends: "./vite.config.js",
                test: {
                    name: "client",
                    browser: {
                        enabled: true,
                        provider: playwright(),
                        instances: [{ browser: "chromium", headless: true }]
                    },
                    include: ["src/**/*.svelte.{test,spec}.{js,ts}"],
                    exclude: ["src/lib/server/**"]
                }
            },

            {
                extends: "./vite.config.js",
                test: {
                    name: "server",
                    environment: "node",
                    include: ["src/**/*.{test,spec}.{js,ts}"],
                    exclude: ["src/**/*.svelte.{test,spec}.{js,ts}"]
                }
            }
        ]
    }
}));
