import { expect, test, type Page } from '@playwright/test'
import { mkdirSync } from 'node:fs'

// A 1×1 PNG.
const PNG_BASE64 =
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=='

async function pasteImage(page: Page) {
  await page.locator('#composer-input').evaluate((input, base64) => {
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
}

async function dropFile(page: Page, name: string, text: string) {
  await page.locator('#composer').evaluate(
    (form, [name, text]) => {
      const data = new DataTransfer()
      data.items.add(new File([text!], name!, { type: 'text/plain' }))
      for (const type of ['dragover', 'drop']) {
        form.dispatchEvent(
          new DragEvent(type, {
            dataTransfer: data,
            bubbles: true,
            cancelable: true,
          }),
        )
      }
    },
    [name, text],
  )
}

test('composer: paste and drop files, remove one, and send them with the message', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login', exact: true }).click()
  const composer = page.locator('#composer-input')
  await composer.click()

  await pasteImage(page)
  await dropFile(page, 'notes.txt', 'hello')
  await dropFile(page, 'spare.log', 'unused')
  const chips = page.locator('[data-od-id="composer-attachment"]')
  await expect(chips).toHaveCount(3)
  // A pasted screenshot gets a thumbnail and a name that says when it was pasted.
  await expect(chips.first().locator('img')).toBeVisible()
  await expect(chips.first()).toContainText(/pasted-.*\.png/)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/composer-attachments.png' })

  await page.getByRole('button', { name: 'Remove spare.log' }).click()
  await expect(chips).toHaveCount(2)

  // Attachments alone can be sent.
  await expect(page.locator('[data-od-id="composer-send"]')).toBeEnabled()
  await composer.fill('Why is this layout broken?')
  await composer.press('Enter')
  await expect(chips).toHaveCount(0)

  const prompts = await page.evaluate(
    () =>
      (
        window as {
          __HARNESS_PROMPTS__?: {
            text: string
            attachments: { name: string; dataBase64: string | null }[]
          }[]
        }
      ).__HARNESS_PROMPTS__,
  )
  const sent = prompts!.at(-1)!
  expect(sent.text).toBe('Why is this layout broken?')
  expect(sent.attachments.map((a) => a.name)).toEqual([
    expect.stringMatching(/^pasted-.*\.png$/),
    'notes.txt',
  ])
  expect(sent.attachments[0]!.dataBase64).toBe(PNG_BASE64)
  expect(sent.attachments[1]!.dataBase64).toBe(btoa('hello'))

  // The conversation shows what was attached.
  await expect(
    page.getByText(
      /Why is this layout broken\?\s+Attached: pasted-.*\.png, notes\.txt/,
    ),
  ).toBeVisible()
  expect(errors).toEqual([])
})
