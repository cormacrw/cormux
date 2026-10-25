import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('app events render as divider markers, one per row', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/?longThread=1')
  await page.getByRole('button', { name: 'OAuth login, Idle' }).click()
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

test('New session clears the view to a greeting; an earlier conversation can be resumed', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/?longThread=2')
  await page.getByRole('button', { name: 'OAuth login, Idle' }).click()
  const timeline = page.locator('#timeline')
  const hero = page.locator('[data-od-id="thread-hero"]')
  const button = page.locator('[data-od-id="composer-new-session"]')
  await expect(timeline.getByText('Answer 1')).toBeVisible()

  await button.click()
  // The earlier conversation leaves the view, and a greeting takes its place.
  await expect(hero).toBeVisible()
  await expect(timeline.getByText('Answer 1')).toHaveCount(0)
  await expect(timeline.getByText('Started a new session')).toHaveCount(0)
  // Nothing new to forget yet.
  await expect(button).toBeDisabled()
  await expect(page.locator('[data-od-id="resume-card"]')).toBeVisible()
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/thread-new-session.png' })

  // Picking a task types a prompt for it, without sending it.
  const composer = page.locator('#composer-input')
  await page.locator('[data-od-id="task-select"]').click()
  await page.getByRole('option', { name: /Draft the 0\.4 changelog/ }).click()
  await expect(composer).toHaveValue(/^Complete #CMUX-2: /)
  await expect(composer).toBeFocused()
  await expect(hero).toBeVisible()

  await composer.fill('Fresh question')
  await page.keyboard.press('Enter')
  await expect(hero).toHaveCount(0)
  await expect(timeline.getByText('Fresh question')).toBeVisible()
  await expect(timeline.getByText(/done\.$/)).toBeVisible({ timeout: 20_000 })
  await expect(button).toBeEnabled()

  // ⌘L starts another; now two earlier conversations, newest first.
  await page.keyboard.press('ControlOrMeta+l')
  await expect(hero).toBeVisible()
  await page.locator('[data-od-id="resume-select"]').click()
  const options = page.getByRole('option')
  await expect(options).toHaveCount(2)
  await expect(options.first()).toContainText('Fresh question')
  // Picking one resumes it straight away.
  await options.nth(1).click()

  // The first conversation is back, with none of the others.
  await expect(hero).toHaveCount(0)
  await expect(timeline.getByText('Answer 1')).toBeVisible()
  await expect(timeline.getByText('Fresh question')).toHaveCount(0)
  expect(
    await page.evaluate(
      () =>
        (window as { __HARNESS_RESUMED__?: { conversationId: string } })
          .__HARNESS_RESUMED__?.conversationId,
    ),
  ).toBe('first')
  expect(errors).toEqual([])
})
