import { expect, test } from '@playwright/test'

test.describe('navigation shortcuts', () => {
  test('⌘1 opens the first thread and ⌘N goes to a new one', async ({
    page,
  }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    await page.goto('/')

    await expect(page.locator('[data-od-id="nav-new-thread"]')).toHaveAttribute(
      'aria-current',
      'page',
    )
    const homebase = page.getByRole('button', { name: /^New thread/ })

    await page.keyboard.press('ControlOrMeta+1')
    await expect(
      page.getByRole('button', { name: /^OAuth login,/ }),
    ).toHaveAttribute('aria-current', 'page')

    // No second workspace: nothing happens.
    await page.keyboard.press('ControlOrMeta+2')
    await expect(
      page.getByRole('button', { name: /^OAuth login,/ }),
    ).toHaveAttribute('aria-current', 'page')

    await page.keyboard.press('ControlOrMeta+n')
    await expect(homebase).toHaveAttribute('aria-current', 'page')

    expect(errors).toEqual([])
  })

  test('⌘G shows and hides Git in a workspace', async ({ page }) => {
    await page.goto('/')
    await expect(
      page.getByRole('button', { name: /^OAuth login,/ }),
    ).toBeVisible()
    // A new thread has no Git; nothing happens.
    await page.keyboard.press('ControlOrMeta+g')
    await page.keyboard.press('ControlOrMeta+1')
    const git = page.locator('#tray-changes')
    await expect(git).toHaveAttribute('aria-pressed', 'false')
    await page.keyboard.press('ControlOrMeta+g')
    await expect(git).toHaveAttribute('aria-pressed', 'true')
    await expect(
      page.getByRole('region', { name: 'Git', exact: true }),
    ).toBeVisible()
    await page.keyboard.press('ControlOrMeta+g')
    await expect(git).toHaveAttribute('aria-pressed', 'false')
  })

  test('⌘O shows and hides Output in a workspace', async ({ page }) => {
    await page.goto('/')
    await expect(
      page.getByRole('button', { name: /^OAuth login,/ }),
    ).toBeVisible()
    await page.keyboard.press('ControlOrMeta+1')
    const output = page.locator('#tray-output')
    await expect(output).toHaveAttribute('aria-pressed', 'false')
    await page.keyboard.press('ControlOrMeta+o')
    await expect(output).toHaveAttribute('aria-pressed', 'true')
    await page.keyboard.press('ControlOrMeta+o')
    await expect(output).toHaveAttribute('aria-pressed', 'false')

    // Clicking the Git or Output tab toggles it the same way.
    await output.click()
    await expect(output).toHaveAttribute('aria-pressed', 'true')
    await output.click()
    await expect(output).toHaveAttribute('aria-pressed', 'false')
    const git = page.locator('#tray-changes')
    await git.click()
    await expect(git).toHaveAttribute('aria-pressed', 'true')
    await git.click()
    await expect(git).toHaveAttribute('aria-pressed', 'false')
  })
})
