import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('usage: a thread shows its tokens, context fill and cost, totalled in the header and on Homebase', async ({
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
  await expect(page.locator('[data-od-id="home-cost"]')).toHaveText(
    '$1.23 spent by agents',
  )
  // It sits on the title's baseline, not the header's bottom edge.
  const titleBox = (await page.locator('#home-title').boundingBox())!
  const costBox = (await page
    .locator('[data-od-id="home-cost"]')
    .boundingBox())!
  const baseline = async (selector: string) =>
    page.locator(selector).evaluate((el) => {
      const probe = document.createElement('span')
      probe.style.display = 'inline-block'
      probe.style.height = '0'
      el.appendChild(probe)
      const y = probe.getBoundingClientRect().top
      probe.remove()
      return y
    })
  expect(
    Math.abs(
      (await baseline('#home-title')) -
        (await baseline('[data-od-id="home-cost"]')),
    ),
  ).toBeLessThan(1.5)
  expect(costBox.x).toBeGreaterThan(titleBox.x + titleBox.width)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({
    path: 'e2e/output/home-cost.png',
    clip: { x: 248, y: 0, width: 1032, height: 90 },
  })
  await page.getByRole('button', { name: 'OAuth login', exact: true }).click()

  await expect(page.locator('[data-od-id="composer-usage"]')).toHaveText(
    '22.3k · 11% · $1.23',
  )
  await expect(page.locator('#thread-tab-th-lead')).toHaveAttribute(
    'title',
    '22.3k tokens in context (11% of 200k) · $1.23 so far',
  )
  await expect(page.locator('[data-od-id="ws-cost"]')).toHaveText('$1.23')
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/thread-usage.png' })
})

test('usage: nothing reported shows no zero meter or cost', async ({
  page,
}) => {
  await page.goto('/')
  await expect(page.locator('[data-od-id="home-cost"]')).toHaveCount(0)
  await page.getByRole('button', { name: 'OAuth login', exact: true }).click()
  await expect(page.locator('[data-od-id="composer-usage"]')).toHaveCount(0)
  await expect(page.locator('[data-od-id="ws-cost"]')).toHaveCount(0)
  await expect(page.locator('#thread-tab-th-lead')).toHaveAttribute(
    'title',
    'Usage not reported',
  )
})
