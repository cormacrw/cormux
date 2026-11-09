import { defineConfig, devices } from '@playwright/test'

// Parallel worktrees each need their own server: one on 1420 would answer for all of them.
const port = Number(process.env.E2E_PORT ?? 1420)
const url = `http://127.0.0.1:${port}`

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: process.env.CI ? 'github' : 'list',
  use: {
    baseURL: url,
    trace: 'on-first-retry',
    colorScheme: 'dark',
    viewport: { width: 1280, height: 800 },
  },
  webServer: {
    command: `pnpm dev --host 127.0.0.1 --port ${port}`,
    url,
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
})
