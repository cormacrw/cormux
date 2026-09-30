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
  await expect(tabs.getByRole('tab')).toHaveCount(4)
  await expect(tabs.getByRole('tab', { name: /^Agent 4/ })).toHaveAttribute(
    'aria-selected',
    'true',
  )
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/thread-tabs.png' })

  // Closing the open tab moves to its neighbour.
  await tabs.getByRole('button', { name: 'Close Agent 4' }).click()
  await expect(tabs.getByRole('tab')).toHaveCount(3)
  await expect(tabs.getByRole('tab', { name: /^Agent 3/ })).toHaveAttribute(
    'aria-selected',
    'true',
  )

  await tabs.getByRole('tab', { name: /^Agent 2/ }).click({ button: 'middle' })
  await expect(tabs.getByRole('tab', { name: /^Agent 2/ })).toHaveCount(0)

  await tabs.getByRole('tab', { name: /^Agent 3/ }).focus()
  await page.keyboard.press('Delete')
  await expect(tabs.getByRole('tab')).toHaveCount(1)
  await expect(tabs.getByRole('tab', { name: /^Lead/ })).toHaveAttribute(
    'aria-selected',
    'true',
  )

  await tabs.getByRole('tab', { name: /^Lead/ }).focus()
  await page.keyboard.press('Delete')
  await expect(tabs.getByRole('tab')).toHaveCount(1)
  expect(errors).toEqual([])
})
