import { installBrowserHarness } from './lib/dev/browser-harness'

installBrowserHarness()
await import('./main')
