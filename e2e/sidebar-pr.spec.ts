import { expect, test, type Page } from '@playwright/test'
import { mkdirSync } from 'node:fs'
import { setScenario } from './harness'

const LONG_BRANCH =
  'feature/very-long-branch-name-for-the-oauth-login-refactor-with-pkce'

function pullRequest(head: string, isDraft = false) {
  return {
    id: 'acme/app#57',
    repoId: 'my-app',
    number: 57,
    title: 'OAuth login',
    payload: {
      num: 57,
      title: 'OAuth login',
      author: 'you',
      head,
      base: 'main',
      updatedAt: '2026-09-29T20:00:00Z',
      checks: 'pass' as const,
      review: 'required' as const,
      files: 3,
      isDraft,
      htmlUrl: 'https://github.com/acme/app/pull/57',
      repoFullName: 'acme/app',
      repoId: 'my-app',
    },
  }
}

async function shootSidebar(page: Page, name: string) {
  mkdirSync('e2e/output', { recursive: true })
  await page.waitForTimeout(400) // let the rows settle
  await page.screenshot({
    path: `e2e/output/${name}.png`,
    clip: { x: 0, y: 140, width: 260, height: 140 },
  })
}

test('a long branch truncates and leaves the PR icon in the corner', async ({
  page,
}) => {
  await setScenario(page, {
    workspaceBranch: LONG_BRANCH,
    pullRequests: [pullRequest(LONG_BRANCH)],
  })
  await page.setViewportSize({ width: 1280, height: 800 })
  await page.goto('/')
  const row = page.getByRole('button', { name: /^OAuth login,/ })
  await expect(row).toHaveAccessibleName(/PR #57 open/)
  const icon = row.locator('[data-od-id="thread-row-pr"]')
  await expect(icon).toHaveAttribute('data-pr-state', 'open')

  // The icon sits flush right inside the row, and the branch gives way to it.
  const rowBox = (await row.boundingBox())!
  const iconBox = (await icon.boundingBox())!
  expect(iconBox.width).toBeGreaterThan(10)
  expect(rowBox.x + rowBox.width - (iconBox.x + iconBox.width)).toBeLessThan(14)
  const branch = row.getByText(LONG_BRANCH)
  const branchBox = (await branch.boundingBox())!
  expect(branchBox.x + branchBox.width).toBeLessThanOrEqual(iconBox.x)
  expect(await branch.evaluate((el) => el.scrollWidth > el.clientWidth)).toBe(
    true,
  )
  // No status dot anywhere in the list.
  await expect(page.locator('.thread-row .status-flash')).toHaveCount(0)
  await shootSidebar(page, 'sidebar-pr-long-branch')
})

test('a draft PR has its own icon, and a row without a PR has none', async ({
  page,
}) => {
  await setScenario(page, {
    secondWorkspace: true,
    pullRequests: [pullRequest('feat/oauth-login', true)],
  })
  await page.setViewportSize({ width: 1280, height: 800 })
  await page.goto('/')
  const row = page.getByRole('button', { name: /^OAuth login,/ })
  await expect(row.locator('[data-od-id="thread-row-pr"]')).toHaveAttribute(
    'data-pr-state',
    'draft',
  )
  await expect(
    page
      .getByRole('button', { name: /^Billing fix,/ })
      .locator('[data-od-id="thread-row-pr"]'),
  ).toHaveCount(0)
  await shootSidebar(page, 'sidebar-pr-short-branch')
})
