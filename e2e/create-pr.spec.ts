import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('Create PR drafts a description with no extra options', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.setViewportSize({ width: 1440, height: 900 })
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await page.locator('[data-od-id="ws-create-pr"]').click()

  const dialog = page.locator('[data-od-id="pr-dialog"]')
  await expect(dialog.getByLabel('Description')).toHaveAttribute(
    'aria-busy',
    'true',
  )
  // Let the open animation settle before capturing the skeleton.
  await page.waitForTimeout(300)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/create-pr-drafting.png' })

  await expect(dialog.getByLabel('Description')).toHaveValue(
    'Lets people sign in with their Google account.',
  )
  await expect(dialog.getByRole('checkbox')).toHaveCount(0)
  const box = await dialog.boundingBox()
  expect(box?.width).toBeGreaterThan(800)
  await page.screenshot({ path: 'e2e/output/create-pr-drafted.png' })
  expect(errors).toEqual([])
})

test('Settings shows the PR prompt, prefilled with the default', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'Settings' }).click()
  const field = page.getByLabel('PR prompt')
  await expect(field).toHaveValue(
    'Write the description for this pull request.',
  )
  await expect(
    page.getByRole('button', { name: 'Reset to default' }),
  ).toHaveCount(0)

  await field.fill('Keep it to one paragraph.')
  await expect(
    page.getByRole('button', { name: 'Reset to default' }),
  ).toBeVisible()
  await page.locator('[data-od-id="settings-pr-prompt"]').screenshot({
    path: 'e2e/output/settings-pr-prompt.png',
  })

  await page.getByRole('button', { name: 'Reset to default' }).click()
  await expect(field).toHaveValue(
    'Write the description for this pull request.',
  )
})
