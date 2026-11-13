import { expect, test } from '@playwright/test'
import { setScenario } from './harness'

test('the palette opens a workspace’s retired thread', async ({ page }) => {
  await setScenario(page, { retiredThread: true })
  await page.goto('/')
  await page.getByRole('button', { name: /Search or run a command/ }).click()
  const dialog = page.getByRole('dialog')
  await page.keyboard.type('Open Reviewer')
  const option = dialog.getByText('Open Reviewer in OAuth login')
  await expect(option).toBeVisible()
  await option.click()
  await expect(dialog).toHaveCount(0)
  await expect(page.getByRole('heading', { name: 'OAuth login' })).toBeVisible()
})

test('the palette finds a thread by its PR number', async ({ page }) => {
  await setScenario(page, {
    pullRequests: [
      {
        id: 'acme/app#57',
        repoId: 'my-app',
        number: 57,
        title: 'OAuth login',
        payload: {
          num: 57,
          title: 'OAuth login',
          author: 'you',
          head: 'feat/oauth-login',
          base: 'main',
          updatedAt: '2026-09-29T20:00:00Z',
          checks: 'pass',
          review: 'required',
          files: 3,
          isDraft: false,
          htmlUrl: 'https://github.com/acme/app/pull/57',
          repoFullName: 'acme/app',
          repoId: 'my-app',
        },
      },
    ],
  })
  await page.goto('/')
  await page.getByRole('button', { name: /Search or run a command/ }).click()
  const dialog = page.getByRole('dialog')
  for (const query of ['#57', 'pr 57']) {
    await page.keyboard.press('ControlOrMeta+A')
    await page.keyboard.type(query)
    await expect(dialog.getByText('Open OAuth login')).toBeVisible()
    await expect(dialog.getByText('PR #57 · Needs Attention')).toBeVisible()
  }
  // Reviewing a PR is no longer a palette command.
  await expect(dialog.getByText(/^Review PR/)).toHaveCount(0)
  await dialog.getByText('Open OAuth login').click()
  await expect(page.getByRole('heading', { name: 'OAuth login' })).toBeVisible()
})
