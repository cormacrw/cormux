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

test('New session keeps the log and adds a marker; ⌘L does the same', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/?longThread=2')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  const timeline = page.locator('#timeline')
  const markers = timeline.getByText('Started a new session')
  const button = page.locator('[data-od-id="composer-new-session"]')

  await button.click()
  await expect(markers).toHaveCount(1)
  // The earlier conversation stays in the thread.
  await expect(timeline.getByText('Answer 1')).toBeVisible()
  // Nothing new to forget yet.
  await expect(button).toBeDisabled()

  await page.locator('#composer-input').fill('Fresh question')
  await page.keyboard.press('Enter')
  await expect(timeline.getByText(/done\.$/).last()).toBeVisible()
  await expect(button).toBeEnabled()
  await page.keyboard.press('ControlOrMeta+l')
  await expect(markers).toHaveCount(2)
  mkdirSync('e2e/output', { recursive: true })
  await timeline.screenshot({ path: 'e2e/output/thread-new-session.png' })
  expect(errors).toEqual([])
})
