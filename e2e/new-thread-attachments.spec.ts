import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

// A 1×1 PNG.
const PNG_BASE64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=='

test('new thread: a pasted image goes with the first message', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'New thread' })).toBeVisible()
  await expect(page.locator('[data-od-id="composer-attach"]')).toBeVisible()

  const composer = page.locator('#composer-input')
  await composer.click()
  await composer.evaluate((input, base64) => {
    const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0))
    const data = new DataTransfer()
    data.items.add(new File([bytes], 'image.png', { type: 'image/png' }))
    input.dispatchEvent(
      new ClipboardEvent('paste', {
        clipboardData: data,
        bubbles: true,
        cancelable: true,
      }),
    )
  }, PNG_BASE64)
  const chips = page.locator('[data-od-id="composer-attachment"]')
  await expect(chips).toHaveCount(1)
  await composer.fill('Why is this layout broken?')
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/new-thread-attachments.png' })

  await composer.press('Enter')
  await expect(
    page.getByText(/Why is this layout broken\?\s+Attached: pasted-.*\.png/),
  ).toBeVisible()
  const prompts = await page.evaluate(() => window.__HARNESS__!.prompts)
  const sent = prompts.at(-1)!
  expect(sent.text).toBe('Why is this layout broken?')
  expect(sent.attachments).toHaveLength(1)
  expect(sent.attachments[0]!.dataBase64).toBe(PNG_BASE64)
  expect(errors).toEqual([])
})
