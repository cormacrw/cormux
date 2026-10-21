import { expect, test } from '@playwright/test'

test('the composer gets focus back after the usual actions', async ({
  page,
}) => {
  await page.addInitScript(() => {
    ;(window as { __HARNESS_LEAD_ENGINE__?: string }).__HARNESS_LEAD_ENGINE__ =
      'claude'
  })
  await page.goto('/')
  const composer = page.locator('#composer-input')

  // Opening a workspace is ready to type.
  await page.getByRole('button', { name: 'OAuth login', exact: true }).click()
  await expect(composer).toBeFocused()

  // Plan and attach controls hand focus back.
  await page.locator('[data-od-id="composer-plan-mode"]').click()
  await expect(composer).toBeFocused()

  // Picking a model returns to the prompt, not the picker's button.
  await page.getByRole('button', { name: /^Model:/ }).click()
  await page.getByRole('menuitemradio', { name: 'Opus' }).click()
  await expect(composer).toBeFocused()

  // Back from another tab by clicking the thread's tab.
  await page.locator('#thread-tab-changes').click()
  await page.locator('#thread-tab-th-lead').click()
  await expect(composer).toBeFocused()

  // A click in the conversation drops focus; typing still lands in the prompt.
  await page.locator('#thread-scroll').click({ position: { x: 20, y: 20 } })
  await expect(composer).not.toBeFocused()
  await page.keyboard.type('hi')
  await expect(composer).toBeFocused()
  await expect(composer).toHaveValue('hi')

  // Ctrl+Tab back into the workspace lands in the prompt too.
  await page.getByRole('button', { name: /^TODOs/ }).click()
  await page.keyboard.press('Control+Tab')
  await expect(composer).toBeFocused()
})

test('arrow keys through the tabs keep focus on the tabs', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login', exact: true }).click()
  await page.locator('#thread-tab-th-lead').focus()
  await page.keyboard.press('ArrowRight')
  await page.keyboard.press('ArrowLeft')
  await expect(page.locator('#thread-tab-th-lead')).toBeFocused()
})
