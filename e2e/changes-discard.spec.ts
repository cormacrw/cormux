import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('Discarding uncommitted changes asks first, one file or all of them', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await page.getByRole('tab', { name: /^Git/ }).click()
  const panel = page.getByRole('tabpanel', { name: /^Git/ })
  const files = panel.getByRole('list', { name: 'Changed files' })
  await expect(files.getByRole('listitem')).toHaveCount(2)

  await files.getByText('session.ts').hover()
  await panel
    .getByRole('button', { name: 'Discard changes to src/auth/session.ts' })
    .click()
  const dialog = page.getByRole('alertdialog')
  await expect(dialog).toContainText('Discard changes to this file?')
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/changes-discard.png' })
  await dialog.getByRole('button', { name: 'Cancel' }).click()
  await expect(files.getByRole('listitem')).toHaveCount(2)

  await panel
    .getByRole('button', { name: 'Discard changes to src/auth/session.ts' })
    .click()
  await page
    .getByRole('alertdialog')
    .getByRole('button', { name: /^Discard/ })
    .click()
  await expect(files.getByRole('listitem')).toHaveCount(1)
  await expect(files).not.toContainText('session.ts')

  await panel.getByRole('button', { name: 'Discard all' }).click()
  await expect(page.getByRole('alertdialog')).toContainText('(1 file)')
  await page
    .getByRole('alertdialog')
    .getByRole('button', { name: /^Discard/ })
    .click()
  await expect(panel.getByText('Clean diff!')).toBeVisible()
})

test('A branch level has no discard', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await page.getByRole('tab', { name: /^Git/ }).click()
  const panel = page.getByRole('tabpanel', { name: /^Git/ })
  await expect(panel.getByRole('button', { name: 'Discard all' })).toBeVisible()
  await panel
    .getByRole('navigation', { name: 'Stack' })
    .getByRole('button', { name: /^feat\/oauth-login/ })
    .click()
  await expect(panel.locator('[data-od-id="changes-target"]')).toHaveText(
    'feat/oauth-login vs feat/oauth-api',
  )
  await expect(panel.getByRole('button', { name: 'Discard all' })).toHaveCount(
    0,
  )
  await expect(
    panel.getByRole('button', { name: /^Discard changes to/ }),
  ).toHaveCount(0)
})
