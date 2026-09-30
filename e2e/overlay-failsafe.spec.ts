import { expect, test } from '@playwright/test'

// Some WebKit installs never report a closing layer's exit animation as finished, so bits-ui
// kept the invisible overlay mounted and the body at pointer-events: none. Simulate that and
// check the app still takes clicks once the dialog closes.
test('the app stays clickable when a dialog exit animation never finishes', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const stuck = {
      finished: new Promise(() => {}),
      pending: false,
      playState: 'running',
      finish() {},
    }
    Element.prototype.getAnimations = function () {
      return this.hasAttribute('data-ending-style')
        ? [stuck as unknown as Animation]
        : []
    }
  })
  await page.goto('/')

  const newWorkspace = page.getByRole('button', {
    name: 'New Workspace',
    exact: true,
  })
  await newWorkspace.click()
  await expect(page.getByRole('dialog')).toBeVisible()
  await page.keyboard.press('Escape')

  // Without the failsafe the overlay stays in place and this click never lands.
  await newWorkspace.click({ timeout: 5000 })
  await expect(
    page.getByRole('dialog', { name: /New workspace/i }),
  ).toBeVisible()
  await page.keyboard.press('Escape')
  await expect
    .poll(() => page.evaluate(() => document.body.style.pointerEvents))
    .not.toBe('none')
})

// The dialog closes cleanly but the body keeps pointer-events: none, so the keyboard
// still works and the mouse does nothing. The failsafe clears it once nothing is open.
test('the app stays clickable when a closed dialog leaves the body locked', async ({
  page,
}) => {
  await page.goto('/')
  const newWorkspace = page.getByRole('button', {
    name: 'New Workspace',
    exact: true,
  })
  await newWorkspace.click()
  await expect(page.getByRole('dialog')).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(page.getByRole('dialog')).toHaveCount(0)

  await page.evaluate(() => (document.body.style.pointerEvents = 'none'))
  await newWorkspace.click({ timeout: 5000 })
  await expect(page.getByRole('dialog')).toBeVisible()
})

test('an open menu keeps the body locked', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await page.locator('[data-od-id="ws-branch"]').click()
  await expect(page.getByRole('menu', { name: 'Switch branch' })).toBeVisible()
  await expect
    .poll(() => page.evaluate(() => document.body.style.pointerEvents))
    .toBe('none')
  await page.waitForTimeout(800)
  expect(await page.evaluate(() => document.body.style.pointerEvents)).toBe(
    'none',
  )
})
