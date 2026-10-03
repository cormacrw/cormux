import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('typing / in a Claude composer offers skills and inserts the pick', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  mkdirSync('e2e/output', { recursive: true })
  await page.goto('/')

  await page
    .getByRole('button', {
      name: 'Why the webhook signature fails, my-app, Idle',
    })
    .click()
  const input = page.locator('#composer-input')
  await input.fill('/')
  const picker = page.getByRole('listbox', { name: 'Skills' })
  await expect(picker.getByRole('option')).toHaveCount(3)
  await page.waitForTimeout(200)
  await page.screenshot({ path: 'e2e/output/skill-picker.png' })

  await input.pressSequentially('rev')
  await expect(picker.getByRole('option')).toHaveCount(1)
  await input.press('Enter')
  await expect(input).toHaveValue('/code-review ')
  await expect(picker).toHaveCount(0)

  // Esc closes the picker without stopping anything.
  await input.fill('/')
  await expect(picker).toBeVisible()
  await input.press('Escape')
  await expect(picker).toHaveCount(0)

  expect(errors).toEqual([])
})
