import { expect, test } from '@playwright/test'
import { setScenario } from './harness'
import { mkdirSync } from 'node:fs'

test('usage: a thread shows its tokens, context fill and cost, totalled in the header', async ({
  page,
}) => {
  await setScenario(page, {
    leadUsage: {
      usedTokens: 22_274,
      contextSize: 200_000,
      costUsd: 1.234,
    },
  })
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()

  // The prompt bar shows only how full the context is.
  await expect(page.locator('[data-od-id="composer-usage"]')).toHaveText('11%')
  await expect(page.locator('[data-od-id="ws-cost"]')).toHaveText('$1.23')
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/thread-usage.png' })
})

test('usage: nothing reported shows no zero meter or cost', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(page.locator('[data-od-id="composer-usage"]')).toHaveCount(0)
  await expect(page.locator('[data-od-id="ws-cost"]')).toHaveCount(0)
})
