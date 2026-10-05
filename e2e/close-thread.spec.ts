import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('thread tabs close with ×, middle-click or Delete; the Lead stays', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  const tabs = page.getByRole('tablist', { name: 'Agent threads' })
  await expect(tabs.getByRole('button', { name: /^Close/ })).toHaveCount(0)

  for (let i = 0; i < 3; i += 1)
    await page.getByRole('button', { name: 'New thread', exact: true }).click()
  const threadTabs = tabs.getByRole('tab')
  await expect(threadTabs).toHaveCount(4)
  await expect(threadTabs.nth(3)).toHaveAttribute('aria-selected', 'true')
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/thread-tabs.png' })

  // Closing the open tab moves to its neighbour.
  const openId = await threadTabs.nth(3).getAttribute('data-thread-id')
  await page.locator(`[data-od-id="thread-close-${openId}"]`).click()
  await expect(threadTabs).toHaveCount(3)
  await expect(threadTabs.nth(2)).toHaveAttribute('aria-selected', 'true')

  await threadTabs.nth(1).click({ button: 'middle' })
  await expect(threadTabs).toHaveCount(2)

  await threadTabs.nth(1).focus()
  await page.keyboard.press('Delete')
  await expect(threadTabs).toHaveCount(1)
  await expect(threadTabs.nth(0)).toHaveAttribute('aria-selected', 'true')

  await threadTabs.nth(0).focus()
  await page.keyboard.press('Delete')
  await expect(threadTabs).toHaveCount(1)
  expect(errors).toEqual([])
})
