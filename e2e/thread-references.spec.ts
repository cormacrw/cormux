import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'
import { setScenario } from './harness'

const entity = (id: number, name: string, url: string) => ({
  id,
  threadId: 'th-lead',
  name,
  url,
  createdAt: '2026-11-07 10:00:00',
})

const ENTITIES = [
  entity(
    1,
    'PR #42: OAuth login with GitHub',
    'https://github.com/acme/my-app/pull/42',
  ),
  entity(2, 'ENG-118 Sign in with GitHub', 'https://linear.app/acme/ENG-118'),
  entity(3, 'Auth flow spec', 'https://docs.google.com/document/d/1abc/edit'),
]

test('attached entities dock beside the conversation and open in the browser', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await setScenario(page, {
    entities: ENTITIES,
    settings: { experimentalReferences: 'true' },
  })
  await page.setViewportSize({ width: 1680, height: 900 })
  await page.goto('/?longThread=3')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(page.locator('#timeline').getByText('Answer 1')).toBeVisible()

  const panel = page.getByRole('complementary', { name: /References/ })
  await expect(panel).toBeVisible()
  await expect(panel.getByRole('link')).toHaveCount(3)
  await expect(panel).toContainText('acme/my-app')
  await expect(panel).toContainText('linear.app')

  // It sits in the gutter, clear of the conversation.
  const conversation = await page.locator('#timeline').boundingBox()
  const box = await panel.boundingBox()
  expect(box!.x).toBeGreaterThanOrEqual(conversation!.x + conversation!.width)

  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({
    path: 'e2e/output/thread-references.png',
    animations: 'disabled',
  })

  await panel.getByRole('link', { name: /PR #42/ }).click()
  await expect
    .poll(() => page.evaluate(() => window.__HARNESS__!.opened))
    .toEqual(['https://github.com/acme/my-app/pull/42'])

  // Narrower, the conversation moves over instead of running under it.
  await page.setViewportSize({ width: 1240, height: 900 })
  await expect(panel).toBeVisible()
  const narrowConversation = await page.locator('#timeline').boundingBox()
  const narrowBox = await panel.boundingBox()
  expect(narrowBox!.x).toBeGreaterThanOrEqual(
    narrowConversation!.x + narrowConversation!.width,
  )
  await page.screenshot({
    path: 'e2e/output/thread-references-narrow.png',
    animations: 'disabled',
  })

  // Too narrow for both, the conversation keeps the room.
  await page.setViewportSize({ width: 900, height: 900 })
  await expect(panel).toBeHidden()
  expect(errors).toEqual([])
})

test('a thread with nothing attached shows no panel', async ({ page }) => {
  await setScenario(page, { settings: { experimentalReferences: 'true' } })
  await page.goto('/?longThread=1')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(page.locator('#timeline').getByText('Answer 1')).toBeVisible()
  await expect(page.locator('[data-od-id="thread-references"]')).toHaveCount(0)
})

test('the panel is off until turned on in Settings › Experimental', async ({
  page,
}) => {
  await setScenario(page, { entities: ENTITIES })
  await page.setViewportSize({ width: 1680, height: 900 })
  await page.goto('/?longThread=1')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(page.locator('#timeline').getByText('Answer 1')).toBeVisible()
  const panel = page.locator('[data-od-id="thread-references"]')
  await expect(panel).toHaveCount(0)

  await page
    .getByRole('navigation', { name: 'Harness' })
    .getByRole('button', { name: /Settings/ })
    .click()
  const section = page.locator('[data-od-id="settings-experimental"]')
  await section.scrollIntoViewIfNeeded()
  const toggle = section.getByRole('switch', { name: 'References' })
  await expect(toggle).not.toBeChecked()
  await toggle.click()
  await expect(toggle).toBeChecked()
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({
    path: 'e2e/output/settings-experimental.png',
    animations: 'disabled',
  })

  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(panel).toBeVisible()
  await expect(panel.getByRole('link')).toHaveCount(3)
})
