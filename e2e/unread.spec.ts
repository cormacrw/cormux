import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('a thread that moved since you last opened it is unread until you open it', async ({
  page,
}) => {
  // Nothing seen yet: every thread's activity is news.
  await page.addInitScript(() => {
    if (!localStorage.getItem('cormux-sidebar-seen'))
      localStorage.setItem(
        'cormux-sidebar-seen',
        JSON.stringify({ since: 0, at: {} }),
      )
  })
  await page.setViewportSize({ width: 1280, height: 800 })
  await page.goto('/')
  const row = page.getByRole('button', { name: /^Why the webhook signature fails,/ })
  await expect(row).toHaveAccessibleName(/^Why the webhook signature fails, unread,/)
  await expect(row).toHaveAttribute('data-unread', 'true')
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/unread-threads.png' })

  await row.click()
  await expect(row).not.toHaveAttribute('data-unread')
  await page.screenshot({ path: 'e2e/output/unread-after-open.png' })

  // Still read after a reload.
  await page.reload()
  await expect(row).not.toHaveAttribute('data-unread')
})
