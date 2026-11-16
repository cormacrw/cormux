import { expect, test, type Page } from '@playwright/test'
import { setScenario } from './harness'

async function sendWithModEnter(page: Page) {
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  const input = page.locator('#composer-input')
  await input.fill('Ship it')
  await input.press('ControlOrMeta+Enter')
}

const current = (page: Page, name: string) =>
  expect(
    page.getByRole('button', { name: new RegExp(`^${name},`) }),
  ).toHaveAttribute('aria-current', 'page')

test('⌘↵ sends, then moves on to the thread that has waited longest', async ({
  page,
}) => {
  await setScenario(page, {
    secondWorkspace: true,
    settings: { sendAndNext: 'oldest' },
  })
  await page.goto('/')
  await sendWithModEnter(page)
  await current(page, 'Billing fix')
})

test('⌘↵ moves on to the newest unread thread', async ({ page }) => {
  await setScenario(page, {
    secondWorkspace: true,
    settings: { sendAndNext: 'newestUnread' },
  })
  // Nothing seen yet, so Billing fix is unread.
  await page.addInitScript(() =>
    localStorage.setItem(
      'cormux-sidebar-seen',
      JSON.stringify({ since: 0, at: {} }),
    ),
  )
  await page.goto('/')
  await sendWithModEnter(page)
  await current(page, 'Billing fix')
})

test('⌘↵ skips snoozed threads, and stays put with nowhere to go', async ({
  page,
}) => {
  await setScenario(page, {
    secondWorkspace: true,
    settings: { sendAndNext: 'oldest' },
  })
  await page.addInitScript(() =>
    localStorage.setItem(
      'cormux-sidebar-snoozed',
      JSON.stringify({ 'ws-billing': Date.now() + 60 * 60 * 1000 }),
    ),
  )
  await page.goto('/')
  await sendWithModEnter(page)
  await expect(page.locator('#composer-input')).toHaveValue('')
  await current(page, 'OAuth login')
})
