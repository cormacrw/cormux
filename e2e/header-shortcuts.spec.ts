import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('workspace header actions have keyboard shortcuts', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  mkdirSync('e2e/output', { recursive: true })
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()

  const createPr = page.getByRole('button', { name: 'Create PR' })
  await expect(createPr).toHaveAttribute('aria-keyshortcuts', 'Meta+P')
  await page.screenshot({ path: 'e2e/output/workspace-header-shortcuts.png' })

  // Wide windows show every action as a button, with no ⋯ menu.
  await expect(
    page.getByRole('button', { name: 'More workspace actions' }),
  ).toHaveCount(0)
  await page.keyboard.press('ControlOrMeta+t')
  await expect
    .poll(() =>
      page.evaluate(
        () =>
          (window as { __HARNESS_OPENED_TERMINAL__?: unknown })
            .__HARNESS_OPENED_TERMINAL__,
      ),
    )
    .toBe('ws-auth')

  await page.keyboard.press('ControlOrMeta+d')
  const teardown = page.getByRole('alertdialog')
  await expect(teardown.getByText('Teardown & delete worktree?')).toBeVisible()
  await page.keyboard.press('Escape')
  await expect(teardown).toHaveCount(0)

  await page.keyboard.press('ControlOrMeta+b')
  await expect(page.getByRole('menu', { name: 'Switch branch' })).toBeVisible()
  await expect(page.getByPlaceholder('Search branches…')).toBeFocused()
  await page.keyboard.press('Escape')
  await expect(page.getByRole('menu')).toHaveCount(0)

  await page.keyboard.press('ControlOrMeta+p')
  const dialog = page.getByRole('dialog', { name: 'Create pull request' })
  await expect(dialog).toBeVisible()
  // Header shortcuts stay off while a dialog is open.
  await page.keyboard.press('ControlOrMeta+d')
  await expect(page.getByRole('alertdialog')).toHaveCount(0)
  await page.keyboard.press('Escape')
  await expect(dialog).toHaveCount(0)

  expect(errors).toEqual([])
})

test('narrow windows fold the secondary actions into ⋯', async ({ page }) => {
  await page.setViewportSize({ width: 700, height: 720 })
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await expect(
    page.getByRole('button', { name: 'Delete workspace' }),
  ).toHaveCount(0)
  await page.keyboard.press('ControlOrMeta+j')
  const menu = page.getByRole('menu')
  await expect(
    menu.getByRole('menuitem', { name: /Open in Terminal/ }),
  ).toBeVisible()
  await expect(
    menu.getByRole('menuitem', { name: /Delete workspace/ }),
  ).toBeVisible()
  await expect(
    menu.getByRole('menuitem', {
      name: /Rebase|Push|Rename|New thread|editor/,
    }),
  ).toHaveCount(0)
  await page.screenshot({ path: 'e2e/output/workspace-header-narrow.png' })
})

test('End scratch has a keyboard shortcut', async ({ page }) => {
  await page.goto('/')
  await page
    .locator('[data-od-id="home-sessions"]')
    .getByRole('button', {
      name: 'Why the webhook signature fails, my-app, Idle',
    })
    .click()
  await expect(page.locator('#view-session')).toBeVisible()
  await page.screenshot({ path: 'e2e/output/scratch-header-shortcuts.png' })

  await page.keyboard.press('ControlOrMeta+e')
  await expect(
    page.getByRole('alertdialog').getByText('End this scratch?'),
  ).toBeVisible()
})
