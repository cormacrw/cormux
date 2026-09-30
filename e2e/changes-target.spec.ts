import { expect, test, type Page } from '@playwright/test'
import { mkdirSync } from 'node:fs'

async function openChanges(page: Page, query = '') {
  await page.goto(`/${query}`)
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await page.getByRole('tab', { name: /^Git/ }).click()
  return page.getByRole('tabpanel', { name: /^Git/ })
}

test('Changes starts on uncommitted work, and each stack level diffs against the branch below', async ({
  page,
}) => {
  const panel = await openChanges(page)
  const stack = panel.getByRole('navigation', { name: 'Stack' })
  const target = panel.locator('[data-od-id="changes-target"]')
  const uncommitted = stack.getByRole('button', {
    name: /^Uncommitted changes/,
  })
  await expect(target).toHaveText('Uncommitted changes')
  await expect(uncommitted).toHaveAttribute('aria-pressed', 'true')

  const login = stack.getByRole('button', { name: /^feat\/oauth-login/ })
  await login.click()
  await expect(target).toHaveText('feat/oauth-login vs feat/oauth-api')
  await expect(login).toHaveAttribute('aria-pressed', 'true')
  await expect(uncommitted).toHaveAttribute('aria-pressed', 'false')

  // The bottom branch diffs against the trunk.
  await stack.getByRole('button', { name: /^feat\/oauth-api/ }).click()
  await expect(target).toHaveText('feat/oauth-api vs main')
  await page.waitForTimeout(400)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/changes-target.png' })

  await uncommitted.click()
  await expect(target).toHaveText('Uncommitted changes')
})

test('Changes without a stack still offers the branch against its base', async ({
  page,
}) => {
  const panel = await openChanges(page, '?stack=none')
  const stack = panel.getByRole('navigation', { name: 'Stack' })
  await stack.getByRole('button', { name: /^feat\/oauth-login/ }).click()
  await expect(panel.locator('[data-od-id="changes-target"]')).toHaveText(
    'feat/oauth-login vs main',
  )
  await page.waitForTimeout(300)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/changes-no-stack.png' })
})

test('Changes shows a loading splash while the diff is fetched', async ({
  page,
}) => {
  const panel = await openChanges(page, '?slowDiff=1')
  // Files already on screen stay put during a refresh.
  await expect(
    panel.getByRole('region', { name: 'Proposed changes' }),
  ).toBeVisible()
  await expect(panel.getByRole('status')).toHaveCount(0)

  await panel
    .getByRole('navigation', { name: 'Stack' })
    .getByRole('button', { name: /^feat\/oauth-login/ })
    .click()
  const splash = panel.getByRole('status')
  await expect(splash).toHaveText('Loading feat/oauth-login vs feat/oauth-api…')
  const target = panel.locator('[data-od-id="changes-target"]')
  await expect(target).toHaveText('feat/oauth-login vs feat/oauth-api')
  // The splash covers the list instead of unmounting it.
  await expect(
    panel.getByRole('region', { name: 'Proposed changes' }),
  ).toHaveCount(1)
  await page.waitForTimeout(400)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/changes-loading.png' })

  await expect(splash).toHaveCount(0)
  await expect(
    panel.getByRole('region', { name: 'Proposed changes' }),
  ).toBeVisible()
  await expect(target).toHaveText('feat/oauth-login vs feat/oauth-api')
})
