import { expect, test, type Page } from '@playwright/test'
import { mkdirSync } from 'node:fs'

async function openStack(page: Page, query = '') {
  await page.goto(`/${query}`)
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await page.getByRole('tab', { name: /^Git/ }).click()
  return page
    .getByRole('tabpanel', { name: /^Git/ })
    .getByRole('navigation', { name: 'Stack' })
}

test('Changes shows the gh-stack stack beside the diff, checks out, pushes and adds branches', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  const panel = await openStack(page)
  await expect(page.getByRole('tab', { name: /^Stack/ })).toHaveCount(0)
  const cards = panel.locator('[data-stack-branch]')
  await expect(cards).toHaveCount(3)
  // Uncommitted changes sit on top of the stack.
  await expect(panel.locator('li').first()).toHaveAttribute(
    'data-stack-uncommitted',
  )
  // Top of the stack first, trunk underneath.
  await expect(cards.nth(0)).toContainText('feat/oauth-ui')
  await expect(cards.nth(0)).toContainText('needs rebase')
  await expect(cards.nth(1)).toContainText('current')
  await expect(cards.nth(1)).toContainText('+96')
  await expect(cards.nth(1)).toContainText('−12')
  await expect(panel.locator('[data-stack-trunk]')).toContainText('main')

  // gh-stack only adds on top, so Add waits for the top branch.
  await expect(panel.getByRole('button', { name: 'Add branch' })).toBeDisabled()
  // Pull requests stay manual: Create PR is unchanged, and Push only pushes.
  await expect(page.getByRole('button', { name: 'Create PR' })).toBeVisible()
  await expect(panel.getByRole('button', { name: 'Submit' })).toHaveCount(0)
  await expect(cards.nth(2)).toContainText('#101')
  const push = panel.getByRole('button', { name: 'Push' })
  await push.click()
  await expect(push).toBeDisabled()
  await expect(push).toBeEnabled()
  mkdirSync('e2e/output', { recursive: true })
  await page.waitForTimeout(300)
  await page.screenshot({ path: 'e2e/output/stack.png' })

  await panel.getByRole('button', { name: 'Check out feat/oauth-ui' }).click()
  await expect(cards.nth(0)).toContainText('current')
  await panel.getByRole('button', { name: 'Add branch' }).click()
  const input = panel.getByLabel(/New branch on top of/)
  await expect(input).toBeFocused()
  await input.fill('feat/oauth-tests')
  await input.press('Enter')
  await expect(cards).toHaveCount(4)
  await expect(
    panel.locator('[data-stack-branch="feat/oauth-tests"]'),
  ).toContainText('current')
  expect(errors).toEqual([])
})

test('Stack starts from an unstacked branch', async ({ page }) => {
  const panel = await openStack(page, '?stack=none')
  await expect(panel).toContainText("feat/oauth-login isn't in a stack")
  await expect(panel.locator('[data-stack-branch]')).toContainText(
    'not stacked',
  )
  await expect(page.getByRole('button', { name: 'Create PR' })).toBeVisible()
  await panel.getByRole('button', { name: 'Add branch' }).click()
  const input = panel.getByLabel(/New branch on top of/)
  await input.fill('bad name')
  await input.press('Enter')
  await expect(panel).toContainText(
    'Use letters, numbers, dashes and slashes only',
  )
  await page.waitForTimeout(200)
  await page.screenshot({ path: 'e2e/output/stack-add.png' })
  await input.fill('feat/oauth-ui')
  await input.press('Enter')
  await expect(panel.locator('[data-stack-branch]')).toHaveCount(2)
})

test('Stack says why the GitHub check failed and can check again', async ({
  page,
}) => {
  const panel = await openStack(page, '?stack=remote')
  await expect(
    panel.locator('[data-od-id="stack-check-failed"]'),
  ).toContainText('failed talking to GitHub')
  await page.evaluate(
    () => ((window as { __stackRemoteUp?: boolean }).__stackRemoteUp = true),
  )
  await panel.getByRole('button', { name: 'Check GitHub' }).click()
  await expect(panel.locator('[data-stack-branch]')).toHaveCount(3)
  await expect(panel.locator('[data-od-id="stack-check-failed"]')).toHaveCount(
    0,
  )
  await expect(panel.getByRole('button', { name: 'Check GitHub' })).toHaveCount(
    0,
  )
})

test('Stack explains how to install gh-stack and still shows the branch', async ({
  page,
}) => {
  const panel = await openStack(page, '?stack=unavailable')
  await expect(panel).toContainText('gh extension install github/gh-stack')
  await expect(panel.getByRole('button', { name: 'Add branch' })).toHaveCount(0)
  await expect(
    panel.locator('[data-stack-branch="feat/oauth-login"]'),
  ).toContainText('vs main')
  await page.waitForTimeout(200)
  await page.screenshot({ path: 'e2e/output/stack-unavailable.png' })
})

test('Create PR targets the branch below, and the header links an existing PR', async ({
  page,
}) => {
  const panel = await openStack(page)
  // feat/oauth-login has no PR yet; its PR goes against the branch below it.
  await page.getByRole('button', { name: 'Create PR' }).click()
  const dialog = page.getByRole('dialog', { name: 'Create draft pull request' })
  await expect(dialog.locator('[data-od-id="pr-route"]')).toHaveText(
    'feat/oauth-login → feat/oauth-api · 3 files +96 −12',
  )
  await page.waitForTimeout(300)
  await page.screenshot({ path: 'e2e/output/stack-create-pr.png' })
  await page.keyboard.press('Escape')

  // feat/oauth-api already has #101 (from gh-stack), so there's nothing to create.
  await panel.getByRole('button', { name: 'Check out feat/oauth-api' }).click()
  await expect(
    page.getByRole('button', { name: 'PR #101 opened' }),
  ).toBeVisible()
  await expect(page.getByRole('button', { name: 'Create PR' })).toHaveCount(0)
})

test('the header links a PR opened outside Cormux for the checked-out branch', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const payload = {
      num: 57,
      title: 'OAuth login',
      author: 'you',
      rel: 'author',
      head: 'feat/oauth-login',
      base: 'main',
      updatedAt: '2026-09-29T20:00:00Z',
      checks: 'pass',
      failing: null,
      review: 'approved',
      additions: 96,
      deletions: 12,
      files: 3,
      isDraft: false,
      htmlUrl: 'https://github.com/acme/app/pull/57',
      repoFullName: 'acme/app',
      repoId: 'my-app',
    }
    ;(window as { __HARNESS_PRS__?: unknown }).__HARNESS_PRS__ = [
      {
        id: 'acme/app#57',
        repoId: 'my-app',
        number: 57,
        title: payload.title,
        payload: JSON.stringify(payload),
      },
    ]
  })
  await page.goto('/?stack=none')
  await page.getByRole('button', { name: /^OAuth login, Idle/ }).click()
  const button = page.getByRole('button', { name: 'PR #57 · Approved' })
  await expect(button).toBeVisible()
  await button.click()
  const opened = await page.evaluate(
    () => (window as { __HARNESS_OPENED__?: string[] }).__HARNESS_OPENED__,
  )
  expect(opened).toEqual(['https://github.com/acme/app/pull/57'])
})

test('Stack shows when a branch needs rebasing, stacked or not', async ({
  page,
}) => {
  const stacked = await openStack(page)
  await expect(
    stacked.locator('[data-stack-branch="feat/oauth-ui"]'),
  ).toContainText('needs rebase')
  await expect(
    stacked.locator('[data-stack-branch="feat/oauth-login"]'),
  ).not.toContainText('needs rebase')

  const solo = await openStack(page, '?stack=behind')
  const branch = solo.locator('[data-stack-branch="feat/oauth-login"]')
  await expect(branch).toContainText('not stacked')
  await expect(branch).toContainText('needs rebase')
  await page.waitForTimeout(200)
  await page.screenshot({ path: 'e2e/output/stack-needs-rebase.png' })
})
