import { expect, test } from '@playwright/test'

test.describe('navigation shortcuts', () => {
  test('⌘1 opens the first workspace and ⌘H goes back to Homebase', async ({
    page,
  }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    await page.goto('/')

    const homebase = page.getByRole('button', { name: /^Homebase/ })
    await expect(homebase).toHaveAttribute('aria-current', 'page')

    await page.keyboard.press('ControlOrMeta+1')
    await expect(
      page.getByRole('button', { name: /^OAuth login,/ }),
    ).toHaveAttribute('aria-current', 'page')

    // No second workspace: nothing happens.
    await page.keyboard.press('ControlOrMeta+2')
    await expect(
      page.getByRole('button', { name: /^OAuth login,/ }),
    ).toHaveAttribute('aria-current', 'page')

    await page.keyboard.press('ControlOrMeta+h')
    await expect(homebase).toHaveAttribute('aria-current', 'page')

    expect(errors).toEqual([])
  })

  test('⌘G opens the Git tab in a workspace', async ({ page }) => {
    await page.goto('/')
    await expect(
      page.getByRole('button', { name: /^OAuth login,/ }),
    ).toBeVisible()
    // Homebase has no Git tab; nothing happens.
    await page.keyboard.press('ControlOrMeta+g')
    await page.keyboard.press('ControlOrMeta+1')
    const git = page.getByRole('tab', { name: /^Git/ })
    await expect(git).toHaveAttribute('aria-selected', 'false')
    await page.keyboard.press('ControlOrMeta+g')
    await expect(git).toHaveAttribute('aria-selected', 'true')
    await expect(page.getByRole('tabpanel', { name: /^Git/ })).toBeVisible()
  })
})
