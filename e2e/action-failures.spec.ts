import { expect, test } from '@playwright/test'
import { setScenario } from './harness'

test('a failed Retry or Skip on a failed setup says why', async ({ page }) => {
  await setScenario(page, {
    failing: {
      retry_workspace_provisioning: 'setup is still running',
      skip_workspace_provisioning_setup: 'nothing left to skip',
    },
  })
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await page.evaluate(() =>
    window.__HARNESS__!.setStatus('OAuth login', 'provisioningFailed'),
  )
  const toasts = page.locator('[data-tone="bad"]')

  const retry = page.getByRole('button', { name: 'Retry', exact: true })
  await retry.click()
  await expect(
    toasts.filter({ hasText: 'setup is still running' }),
  ).toBeVisible()
  await expect(retry).toBeEnabled()

  await page.getByRole('button', { name: 'Skip and continue' }).click()
  await expect(toasts.filter({ hasText: 'nothing left to skip' })).toBeVisible()
})

test('Git says the diff didn’t load rather than that it’s clean', async ({
  page,
}) => {
  await setScenario(page, {
    failing: {
      subscribe_diffs: 'no worktree',
      refresh_workspace_diff: 'not a git repository',
    },
  })
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await page.locator('#tray-changes').click()
  const failed = page.locator('[data-od-id="changes-refresh-failed"]')
  await expect(failed).toContainText('Couldn’t load the diff')
  await expect(failed).toContainText('not a git repository')
  await expect(page.getByText('Clean diff!')).toHaveCount(0)
})

test('Settings › Agents says why it could not check, not "Not installed"', async ({
  page,
}) => {
  await setScenario(page, { failing: { detect_engines: 'PATH unreadable' } })
  await page.goto('/')
  await page.getByRole('button', { name: 'Settings' }).click()
  const engines = page.locator('[data-od-id="settings-default-engine"]')
  await expect(engines.getByText('PATH unreadable')).toBeVisible()
  await expect(engines.getByText('Not installed')).toHaveCount(0)
})
