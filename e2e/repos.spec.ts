import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test.describe('repos', () => {
  test('lists repos above workspaces, pulls the default branch, and sets it per repo', async ({
    page,
  }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    mkdirSync('e2e/output', { recursive: true })
    await page.goto('/')

    const nav = page.getByRole('navigation', { name: 'Harness' })
    const repoList = nav.getByRole('list', { name: 'Repos' })
    const workspaceList = nav.getByRole('list', { name: /Workspaces/ })
    await expect(repoList.getByText('my-app')).toBeVisible()
    expect((await repoList.boundingBox())!.y).toBeLessThan(
      (await workspaceList.boundingBox())!.y,
    )

    const pull = repoList.getByRole('button', { name: 'Pull main for my-app' })
    await pull.click()
    await expect(pull).toBeDisabled()
    await expect(pull).toBeEnabled()
    await page.screenshot({ path: 'e2e/output/repos-sidebar.png' })

    await page.getByRole('button', { name: 'Settings' }).click()
    await expect(page.getByText('Default base branch')).toHaveCount(0)
    await page.getByRole('button', { name: 'Configure my-app' }).click()
    const field = page.getByRole('button', { name: 'Default branch' })
    await expect(field).toHaveText('main')
    await field.click()
    await page.getByRole('option', { name: 'develop' }).click()
    await expect(field).toHaveText('develop')
    await expect(
      repoList.getByRole('button', { name: 'Pull develop for my-app' }),
    ).toBeVisible()
    const single = page.getByRole('checkbox', {
      name: 'Run one instance at a time',
    })
    await expect(single).not.toBeChecked()
    await single.click()
    await expect(single).toBeChecked()
    await page.screenshot({ path: 'e2e/output/repos-settings.png' })

    expect(errors).toEqual([])
  })

  test('opens a repo page with workspaces, branches, and local delete', async ({
    page,
  }) => {
    mkdirSync('e2e/output', { recursive: true })
    await page.addInitScript(() => {
      ;(window as { __HARNESS_PRS__?: unknown }).__HARNESS_PRS__ = [
        {
          id: 'acme/app#12',
          repoId: 'my-app',
          number: 12,
          title: 'Tune the accent palette',
          payload: JSON.stringify({
            num: 12,
            title: 'Tune the accent palette',
            author: 'you',
            rel: 'author',
            head: 'feat/colors',
            base: 'main',
            updatedAt: '2026-09-29T20:00:00Z',
            checks: 'pass',
            failing: null,
            review: 'approved',
            additions: 10,
            deletions: 2,
            files: 1,
            isDraft: false,
            htmlUrl: 'https://github.com/acme/app/pull/12',
            repoFullName: 'acme/app',
            repoId: 'my-app',
          }),
        },
      ]
    })
    await page.goto('/')
    await page.getByRole('button', { name: 'Open my-app' }).click()

    const pageRoot = page.locator('[data-od-id="repo-page"]')
    await expect(
      pageRoot.getByRole('heading', { name: 'my-app' }),
    ).toBeVisible()
    await expect(pageRoot.getByText('/tmp/cormux-fixture/my-app')).toBeVisible()
    await expect(pageRoot.getByText('/tmp/cormux-fixture/oauth')).toBeVisible()
    await expect(
      pageRoot.getByRole('button', { name: /^OAuth login/ }),
    ).toBeVisible()
    await expect(
      pageRoot.getByText('Why the webhook signature fails'),
    ).toBeVisible()

    const colors = pageRoot.locator('[data-od-id="repo-branch-feat/colors"]')
    await expect(
      colors.getByRole('button', {
        name: 'Open pull request #12 on GitHub, Tune the accent palette',
      }),
    ).toBeVisible()
    await expect(colors.getByText('PR #12')).toBeVisible()
    await expect(
      pageRoot.getByRole('button', { name: 'Delete main, Default branch' }),
    ).toBeDisabled()
    await expect(
      pageRoot.getByRole('button', {
        name: 'Delete feat/oauth-login, Checked out by OAuth login',
      }),
    ).toBeDisabled()

    await page.screenshot({ path: 'e2e/output/repo-page.png' })
    await colors.getByRole('button', { name: 'Delete feat/colors' }).click()
    await expect(colors).toHaveCount(0)
  })

  test('shows how far the default branch is from origin', async ({ page }) => {
    await page.addInitScript(() => {
      ;(window as { __HARNESS_REPO_GIT__?: unknown }).__HARNESS_REPO_GIT__ = [
        { repoId: 'my-app', behind: 3, ahead: 1 },
      ]
    })
    await page.goto('/')

    const repoList = page
      .getByRole('navigation', { name: 'Harness' })
      .getByRole('list', { name: 'Repos' })
    await expect(
      repoList.getByText('3 behind, 1 ahead of origin/main'),
    ).toBeAttached()
    await expect(repoList.getByText('↓3')).toBeVisible()
    await expect(repoList.getByText('↑1')).toBeVisible()
    await expect(
      repoList.getByRole('button', { name: 'Pull main for my-app' }),
    ).toHaveAttribute('title', 'Pull 3 commits into main')
  })
})
