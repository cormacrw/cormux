import { expect, test } from '@playwright/test'

test('Esc stops the agent; ↑/↓ cycle past prompts from an empty composer', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/?longThread=2')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  const input = page.locator('#composer-input')
  const stop = page.locator('[data-od-id="composer-stop"]')

  await input.fill('First prompt')
  await page.keyboard.press('Enter')
  await expect(stop).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(stop).toHaveCount(0)

  await input.fill('Second prompt')
  await page.keyboard.press('Enter')
  await expect(stop).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(stop).toHaveCount(0)

  await input.focus()
  await page.keyboard.press('ArrowUp')
  await expect(input).toHaveValue('Second prompt')
  await page.keyboard.press('ArrowUp')
  await expect(input).toHaveValue('First prompt')
  await page.keyboard.press('ArrowDown')
  await expect(input).toHaveValue('Second prompt')
  await page.keyboard.press('ArrowDown')
  await expect(input).toHaveValue('')

  // Once edited, the arrows move the caret instead.
  await page.keyboard.press('ArrowUp')
  await page.keyboard.type('!')
  await page.keyboard.press('ArrowUp')
  await expect(input).toHaveValue('Second prompt!')
  expect(errors).toEqual([])
})
