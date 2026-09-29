import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('app events render as divider markers, one per row', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/?longThread=1')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  const timeline = page.locator('#timeline')
  const markers = timeline.getByTitle(/Checked out in this worktree/)
  await expect(markers).toHaveCount(3)
  await expect(markers.first()).toContainText('Switched to feat/test')
  // No card around them.
  await expect(timeline.locator('[data-slot="card"]')).toHaveCount(0)
  mkdirSync('e2e/output', { recursive: true })
  await timeline.screenshot({ path: 'e2e/output/thread-events.png' })
  expect(errors).toEqual([])
})
