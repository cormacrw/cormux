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

  test('⌘G shows and hides Git in a workspace', async ({ page }) => {
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
    await page.keyboard.press('ControlOrMeta+g')
    await expect(git).toHaveAttribute('aria-selected', 'false')
  })

  test('⌘O shows and hides Output in a workspace', async ({ page }) => {
    await page.goto('/')
    await expect(
      page.getByRole('button', { name: /^OAuth login,/ }),
    ).toBeVisible()
    await page.keyboard.press('ControlOrMeta+1')
    const output = page.locator('#thread-tab-output')
    await expect(output).toHaveAttribute('aria-selected', 'false')
    await page.keyboard.press('ControlOrMeta+o')
    await expect(output).toHaveAttribute('aria-selected', 'true')
    await page.keyboard.press('ControlOrMeta+o')
    await expect(output).toHaveAttribute('aria-selected', 'false')

    // Clicking the Git or Output tab toggles it the same way.
    await output.click()
    await expect(output).toHaveAttribute('aria-selected', 'true')
    await output.click()
    await expect(output).toHaveAttribute('aria-selected', 'false')
    const git = page.locator('#thread-tab-changes')
    await git.click()
    await expect(git).toHaveAttribute('aria-selected', 'true')
    await git.click()
    await expect(git).toHaveAttribute('aria-selected', 'false')
  })
})

test('⇧⌘] and ⇧⌘[ move through the workspace tabs, and ⇧⌘N starts a thread', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login', exact: true }).click()
  const selected = page.locator('[role="tab"][aria-selected="true"]')
  await expect(selected).toHaveId('thread-tab-th-lead')

  await page.keyboard.press('ControlOrMeta+Shift+BracketRight')
  await expect(selected).toHaveId('thread-tab-changes')
  await page.keyboard.press('ControlOrMeta+Shift+BracketRight')
  await expect(selected).toHaveId('thread-tab-output')
  // It wraps around both ends.
  await page.keyboard.press('ControlOrMeta+Shift+BracketRight')
  await expect(selected).toHaveId('thread-tab-th-lead')
  await page.keyboard.press('ControlOrMeta+Shift+BracketLeft')
  await expect(selected).toHaveId('thread-tab-output')

  const threadTabs = page.locator('#thread-tabs [role="tab"][data-thread-id]')
  await expect(threadTabs).toHaveCount(1)
  await page.keyboard.press('ControlOrMeta+Shift+n')
  await expect(threadTabs).toHaveCount(2)
})

test('⇧⌘P switches plan mode on a Claude thread', async ({ page }) => {
  await page.addInitScript(() => {
    ;(window as { __HARNESS_LEAD_ENGINE__?: string }).__HARNESS_LEAD_ENGINE__ =
      'claude'
  })
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login', exact: true }).click()
  const toggle = page.locator('[data-od-id="composer-plan-mode"]')
  await expect(toggle).toHaveAttribute('aria-pressed', 'false')
  await page.keyboard.press('ControlOrMeta+Shift+p')
  await expect(toggle).toHaveAttribute('aria-pressed', 'true')
  await page.keyboard.press('ControlOrMeta+Shift+p')
  await expect(toggle).toHaveAttribute('aria-pressed', 'false')
})
