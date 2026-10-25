import { expect, test } from '@playwright/test'

test('a pinned tray sits beside the conversation; closing it unpins it', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1400, height: 900 })
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login, Idle' }).click()
  const git = page.getByRole('tab', { name: /^Git/ })
  const pane = page.locator('[data-od-id="git-slide-out"]')
  const composer = page.locator('#composer-input')

  await page.keyboard.press('ControlOrMeta+g')
  await page
    .getByRole('button', { name: 'Pin Git beside the conversation' })
    .click()
  await expect(pane).toHaveAttribute('data-pinned', 'true')
  await expect(pane).toBeVisible()
  // The conversation stays usable beside it, with no scrim over it.
  await composer.fill('still typing')
  await expect(composer).toHaveValue('still typing')
  const paneBox = (await pane.boundingBox())!
  const composerBox = (await composer.boundingBox())!
  expect(composerBox.x + composerBox.width).toBeLessThanOrEqual(paneBox.x)

  // The pin survives a reload.
  await page.reload()
  await page.getByRole('button', { name: 'OAuth login, Idle' }).click()
  await expect(pane).toHaveAttribute('data-pinned', 'true')

  // Output takes the pinned spot while Git is pinned.
  await page.keyboard.press('ControlOrMeta+o')
  const output = page.locator('[data-od-id="output-slide-out"]')
  await expect(output).toHaveAttribute('data-pinned', 'true')
  await expect(pane).toHaveAttribute('data-pinned', 'false')

  // Closing a pinned tray unpins it, so the next open slides out over the conversation.
  await page.keyboard.press('ControlOrMeta+o')
  await expect(output).toHaveAttribute('data-pinned', 'false')
  await expect(output).toHaveAttribute('data-open', 'false')
  await page.keyboard.press('ControlOrMeta+g')
  await expect(pane).toHaveAttribute('data-pinned', 'false')
  await expect(pane).toHaveAttribute('data-open', 'true')
  await expect(git).toHaveAttribute('aria-selected', 'true')
  await page.keyboard.press('Escape')
  await expect(git).toHaveAttribute('aria-selected', 'false')

  // Unpinning from the pin button leaves it open as a slide-out.
  await page.keyboard.press('ControlOrMeta+g')
  await page
    .getByRole('button', { name: 'Pin Git beside the conversation' })
    .click()
  await page.getByRole('button', { name: 'Unpin Git' }).click()
  await expect(pane).toHaveAttribute('data-pinned', 'false')
  await expect(git).toHaveAttribute('aria-selected', 'true')
})
