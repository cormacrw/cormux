import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('the Git tab counts fall away when the changes are committed', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  const gitTab = page.getByRole('tab', { name: /^Git/ })
  const counts = gitTab.locator('.font-mono')
  await expect(counts).toHaveText(/^\+\d+\s*−\d+$/)
  const before = await gitTab.getAttribute('aria-label')
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/change-counts.png' })

  // Committing from a terminal, without opening the Git tab.
  await page.evaluate(() =>
    (
      window as unknown as { __HARNESS_COMMIT__: () => void }
    ).__HARNESS_COMMIT__(),
  )
  await expect(gitTab).not.toHaveAttribute('aria-label', before ?? '')
  await expect(counts).toHaveCount(0, { timeout: 10_000 })
})
