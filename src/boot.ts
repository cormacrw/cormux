// Vite drops this branch from production builds, so the harness and its fixtures never ship.
if (import.meta.env.DEV) {
  const { installBrowserHarness } = await import('./lib/dev/browser-harness')
  installBrowserHarness()
}
await import('./main')
