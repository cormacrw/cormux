import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('usage: a thread shows its tokens, context fill and cost, totalled in the header', async ({
  page,
}) => {
  await page.addInitScript(() => {
    ;(window as { __HARNESS_LEAD_USAGE__?: unknown }).__HARNESS_LEAD_USAGE__ = {
      usedTokens: 22_274,
      contextSize: 200_000,
      costUsd: 1.234,
    }
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

test('usage: the prompt bar shows how the context has grown', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const usage = (seq: number, used_tokens: number) => ({
      id: 9700 + seq,
      threadId: 'th-lead',
      seq,
      kind: 'usage',
      payload: JSON.stringify({
        type: 'usage',
        used_tokens,
        context_size: 200_000,
        cost_usd: 0.1,
      }),
      createdAt: '2026-09-29 12:00:00',
    })
    const win = window as {
      __HARNESS_LEAD_USAGE__?: unknown
      __HARNESS_TIMELINE__?: unknown
    }
    win.__HARNESS_LEAD_USAGE__ = {
      usedTokens: 30_000,
      contextSize: 200_000,
      costUsd: 0.3,
    }
    win.__HARNESS_TIMELINE__ = [
      usage(1, 10_000),
      usage(2, 20_000),
      usage(3, 30_000),
    ]
  })
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(
    page.locator('[data-od-id="composer-usage-trend"]'),
  ).toBeVisible()
})
