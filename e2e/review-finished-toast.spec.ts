import { expect, test } from '@playwright/test'
import { setScenario } from './harness'

test('a review whose findings were already in raises no "Review finished" toast on launch', async ({
  page,
}) => {
  await setScenario(page, {
    reviewFindings: [
      {
        id: 'f1',
        workspaceId: 'ws-auth',
        severity: 'blocking',
        title: 'GitHub sessions skip the expiry check',
        file: null,
        line: null,
        explanation: 'Validate the expiry before returning it.',
        status: 'open',
        commitSha: null,
        sentToThreadId: null,
      },
    ],
  })
  await page.goto('/')
  await expect(
    page.getByRole('button', { name: /^OAuth login,/ }),
  ).toBeVisible()
  await page.waitForFunction(() => window.__HARNESS__?.showToast)
  // Give a toast raised by the first snapshots time to show.
  await page.waitForTimeout(500)
  // Counted once: a toast that did show would pass a retrying check by timing out.
  const toasts = page.locator('[data-sonner-toast]', {
    hasText: /Review .*finished/,
  })
  expect(await toasts.count()).toBe(0)
})
