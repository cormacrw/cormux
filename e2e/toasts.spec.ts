import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('toasts are filled green, red or yellow by tone', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/')
  await page.waitForFunction(
    () => '__HARNESS_SHOW_TOAST__' in (window as object),
  )
  await page.evaluate(() => {
    const show = (
      window as unknown as {
        __HARNESS_SHOW_TOAST__: (payload: unknown) => void
      }
    ).__HARNESS_SHOW_TOAST__
    show({
      tone: 'ok',
      parts: [
        { type: 'text', value: 'Reviewer started on ' },
        { type: 'code', value: '#482' },
      ],
    })
    show({
      tone: 'bad',
      parts: [{ type: 'text', value: 'Setup failed in todo-example' }],
    })
    show({
      tone: 'default',
      parts: [
        {
          type: 'text',
          value: 'Restart the app to run code from the new branch.',
        },
      ],
    })
  })

  const bg = (tone: string) =>
    page
      .locator(`[data-tone="${tone}"]`)
      .evaluate((el) => getComputedStyle(el).backgroundColor)
  await expect(page.locator('[data-tone="default"]')).toBeVisible()
  expect(await bg('ok')).not.toBe('rgba(0, 0, 0, 0)')
  expect(
    new Set([await bg('ok'), await bg('bad'), await bg('default')]).size,
  ).toBe(3)

  // Sonner stacks toasts; hover expands them so all three are in the shot.
  await page.locator('[data-tone="default"]').hover()
  await page.waitForTimeout(400)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/toasts.png' })
  expect(errors).toEqual([])
})

test('clicking a toast opens its workspace thread', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/')
  await page.waitForFunction(
    () => '__HARNESS_SHOW_TOAST__' in (window as object),
  )
  await page.evaluate(() => {
    ;(
      window as unknown as {
        __HARNESS_SHOW_TOAST__: (payload: unknown) => void
      }
    ).__HARNESS_SHOW_TOAST__({
      tone: 'ok',
      workspaceId: 'ws-auth',
      threadId: 'th-lead',
      parts: [{ type: 'text', value: 'Opened draft PR #12' }],
    })
  })

  await page.getByRole('button', { name: 'Opened draft PR #12' }).click()
  await expect(page.getByRole('heading', { name: 'OAuth login' })).toBeVisible()
  await expect(page.locator('#thread-panel')).toBeVisible()
  expect(errors).toEqual([])
})
