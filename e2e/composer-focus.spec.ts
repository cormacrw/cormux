import { expect, test } from '@playwright/test'

test('the composer gets focus back after the usual actions', async ({
  page,
}) => {
  await page.goto('/')
  const composer = page.locator('#composer-input')

  // Opening a workspace is ready to type.
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(composer).toBeFocused()

  // Picking a model returns to the prompt, not the picker's button.
  await page.getByRole('button', { name: /^Model:/ }).click()
  await page.getByRole('menuitemradio', { name: 'Opus' }).click()
  await expect(composer).toBeFocused()

  // Back from a slide-out.
  await page.locator('#tray-changes').click()
  await page.keyboard.press('Escape')
  await expect(composer).toBeFocused()

  // A click in the conversation drops focus; typing still lands in the prompt.
  await page.locator('#thread-scroll').click({ position: { x: 20, y: 20 } })
  await expect(composer).not.toBeFocused()
  await page.keyboard.type('hi')
  await expect(composer).toBeFocused()
  await expect(composer).toHaveValue('hi')

  // Ctrl+Tab back into the workspace lands in the prompt too.
  await page
    .getByRole('navigation', { name: 'Harness' })
    .getByRole('button', { name: /^New thread/ })
    .click()
  await page.keyboard.press('Control+Tab')
  await expect(composer).toBeFocused()
})
