import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests/e2e",
  timeout: 30_000,
  use: {
    baseURL: "http://127.0.0.1:5173",
    trace: "on-first-retry",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
  webServer: [
    {
      command:
        "RAG_DEBUGGER_STORAGE_BACKEND=memory RAG_DEBUGGER_API_PORT=18080 RAG_DEBUGGER_PUBLIC_API_BASE_URL=http://127.0.0.1:18080 RAG_DEBUGGER_WEB_ORIGIN=http://127.0.0.1:5173 cargo run -p rag-debugger-api",
      cwd: "../..",
      url: "http://127.0.0.1:18080/healthz",
      reuseExistingServer: true,
      timeout: 120_000,
    },
    {
      command:
        "VITE_API_BASE_URL=http://127.0.0.1:18080 npm run dev -- --port 5173",
      url: "http://127.0.0.1:5173",
      reuseExistingServer: true,
    },
  ],
});
