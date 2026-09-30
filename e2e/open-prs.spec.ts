import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

const pr = (
  num: number,
  title: string,
  extra: { checks: string; isDraft?: boolean; rel?: string },
) => ({
  id: `acme/app#${num}`,
  repoId: 'my-app',
  number: num,
  title,
  payload: JSON.stringify({
    num,
    title,
    author: extra.rel === 'author' ? 'you' : 'maya-r',
    rel: extra.rel ?? 'review',
    head: `maya/pr-${num}`,
    base: 'main',
    updatedAt: '2026-09-29T20:00:00Z',
    checks: extra.checks,
    failing: null,
    review: extra.isDraft ? 'draft' : 'required',
    additions: 120,
    deletions: 14,
    files: 4,
    isDraft: extra.isDraft ?? false,
    htmlUrl: `https://github.com/acme/app/pull/${num}`,
    repoFullName: 'acme/app',
    repoId: 'my-app',
  }),
})

test('PR rows: status icon colour, title opens GitHub, glasses starts a review', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.addInitScript(
    (prs) => {
      ;(window as { __HARNESS_PRS__?: unknown }).__HARNESS_PRS__ = prs
    },
    [
      pr(482, 'Retry Stripe webhooks with exponential backoff', {
        checks: 'pass',
      }),
      pr(479, 'Move the session store to Redis', { checks: 'running' }),
      pr(471, 'Migrate settings routes to Svelte 5 runes', {
        checks: 'none',
        isDraft: true,
        rel: 'author',
      }),
    ],
  )
  await page.goto('/')

  const section = page.locator('[data-od-id="open-prs"]')
  const row = (num: number) => section.locator(`[data-od-id="pr-row-${num}"]`)
  await expect(row(482)).toBeVisible()

  const iconClass = (num: number) =>
    row(num).locator('svg').first().getAttribute('class')
  expect(await iconClass(482)).toContain('text-emerald-600')
  expect(await iconClass(479)).toContain('text-amber-500')
  expect(await iconClass(471)).toContain('text-muted-foreground')

  await expect(section.getByText('Awaiting review')).toHaveCount(0)
  await expect(row(471).getByText(/Checks/)).toHaveCount(0)
  await expect(row(479).getByText('Checks running').first()).toBeAttached()

  mkdirSync('e2e/output', { recursive: true })
  await section.screenshot({ path: 'e2e/output/open-prs.png' })

  await row(482)
    .getByRole('link', { name: /Retry Stripe webhooks/ })
    .click()
  expect(
    await page.evaluate(
      () => (window as { __HARNESS_OPENED__?: string[] }).__HARNESS_OPENED__,
    ),
  ).toEqual(['https://github.com/acme/app/pull/482'])

  const review = row(482).getByRole('button', {
    name: 'Review #482 in a new workspace',
  })
  await review.hover()
  await expect(page.locator('[data-slot="tooltip-content"]')).toHaveText(
    'Review #482 in a new workspace',
  )
  await page.screenshot({ path: 'e2e/output/open-prs-tooltip.png' })
  expect(errors).toEqual([])
})
