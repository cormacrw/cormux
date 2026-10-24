import { defineConfig, devices } from '@playwright/test'

export default defineConfig({
  testDir: './e2e',
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: process.env.CI ? 'github' : 'list',
  use: {
    baseURL: 'http://127.0.0.1:1420',
    trace: 'on-first-retry',
    colorScheme: 'dark',
    viewport: { width: 1280, height: 800 },
    // Headless Chromium renders WebGL in software, so an animated background would
    // slow every spec down. Specs about the background clear this themselves.
    storageState: {
      cookies: [],
      origins: [
        {
          origin: 'http://127.0.0.1:1420',
          localStorage: [{ name: 'cormux-background-animate', value: '0' }],
        },
      ],
    },
  },
  webServer: {
    command: 'pnpm dev --host 127.0.0.1',
    url: 'http://127.0.0.1:1420',
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
  projects: [{ name: 'chromium', use: { ...devices['Desktop Chrome'] } }],
})
