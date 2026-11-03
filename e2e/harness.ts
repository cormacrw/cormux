import type { Page } from '@playwright/test'
import type { HarnessScenario } from '../src/lib/dev/harness/scenario'

/** How the next load of `page` should start. Call before `goto`; repeat calls merge. */
export function setScenario(page: Page, scenario: HarnessScenario) {
  return page.addInitScript((next) => {
    window.__HARNESS_SCENARIO__ = { ...window.__HARNESS_SCENARIO__, ...next }
  }, scenario)
}
