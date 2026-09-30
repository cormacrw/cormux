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
})
