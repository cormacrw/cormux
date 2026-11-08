import { expect, test } from '@playwright/test'

test('Esc stops the agent; ↑/↓ cycle past prompts from an empty composer', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/?longThread=2')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  const input = page.locator('#composer-input')
  const stop = page.locator('[data-od-id="composer-stop"]')

  await input.fill('First prompt')
  await page.keyboard.press('Enter')
  await expect(input).toBeFocused()
  await expect(input).toHaveValue('')
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

  await input.fill('Clicked send')
  await page.locator('[data-od-id="composer-send"]').click()
  await expect(input).toBeFocused()
  await expect(input).toHaveValue('')

  expect(errors).toEqual([])
})

test('Esc slides a tray away before it stops the agent; Ctrl+` toggles Output', async ({
  page,
}) => {
  await page.goto('/?longThread=2')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  const input = page.locator('#composer-input')
  const stop = page.locator('[data-od-id="composer-stop"]')
  const git = page.locator('#tray-changes')

  await input.fill('Keep going')
  await page.keyboard.press('Enter')
  await expect(stop).toBeVisible()

  await page.keyboard.press('ControlOrMeta+g')
  await expect(git).toHaveAttribute('aria-pressed', 'true')
  await page.keyboard.press('Escape')
  await expect(git).toHaveAttribute('aria-pressed', 'false')
  await expect(stop).toBeVisible()
  await expect(input).toBeFocused()

  await page.keyboard.press('Escape')
  await expect(stop).toHaveCount(0)

  const output = page.locator('[data-od-id="output-slide-out"]')
  await page.keyboard.press('Control+`')
  await expect(output).toHaveAttribute('data-open', 'true')
  await page.keyboard.press('Control+`')
  await expect(output).toHaveAttribute('data-open', 'false')
})
