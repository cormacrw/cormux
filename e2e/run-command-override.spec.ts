import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

async function openOutput(page: import('@playwright/test').Page) {
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login', exact: true }).click()
  await page.locator('#thread-tab-output').click()
}

test('Output: override the repo’s run command for one workspace, then go back to it', async ({
  page,
}) => {
  await page.addInitScript(() => {
    ;(window as { __HARNESS_RUN_COMMAND__?: string }).__HARNESS_RUN_COMMAND__ =
      'pnpm dev'
  })
  await openOutput(page)
  const command = page.locator('[data-od-id="run-command"]')
  await expect(command).toHaveText(/^\s*pnpm dev\s*$/)

  await page
    .getByRole('button', { name: 'Edit the run command for this workspace' })
    .click()
  const input = page.getByRole('textbox', {
    name: 'Run command for this workspace',
  })
  await expect(input).toBeFocused()
  await expect(input).toHaveValue('pnpm dev')

  // Esc keeps the repo's command.
  await input.fill('nope')
  await input.press('Escape')
  await expect(command).toContainText('pnpm dev')
  await expect(command).not.toContainText('this workspace')

  await page
    .getByRole('button', { name: 'Edit the run command for this workspace' })
    .click()
  await input.fill('pnpm dev --port 4000')
  await input.press('Enter')
  await expect(command).toContainText('pnpm dev --port 4000')
  await expect(command).toContainText('this workspace')
  await expect(
    page.getByText('Run starts pnpm dev --port 4000 in this worktree.'),
  ).toBeVisible()
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/run-command-override.png' })

  await page.getByRole('button', { name: "Use the repo's run command" }).click()
  await expect(command).not.toContainText('this workspace')
  await expect(command).toContainText('pnpm dev')
})

test('Output: a workspace can have a run command when its repo has none', async ({
  page,
}) => {
  await openOutput(page)
  await expect(page.getByText(/No run command for/)).toBeVisible()
  await page.getByRole('button', { name: 'Just for this workspace' }).click()
  const input = page.getByRole('textbox', {
    name: 'Run command for this workspace',
  })
  await expect(input).toBeFocused()
  await input.fill('make serve')
  await input.press('Enter')
  await expect(page.locator('[data-od-id="run-command"]')).toContainText(
    'make serve',
  )
  await expect(page.getByText(/No run command for/)).toHaveCount(0)
  await expect(
    page.getByRole('button', { name: 'Run', exact: true }),
  ).toBeEnabled()
})
