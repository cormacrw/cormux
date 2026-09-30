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
