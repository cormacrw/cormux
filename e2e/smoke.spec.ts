import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test.describe('browser harness UI', () => {
  test('opens New workspace and keeps the footer on the dialog', async ({
    page,
  }) => {
    await page.goto('/?slowReply=1')
    await expect(page.locator('html')).toHaveAttribute(
      'data-cormux-harness',
      'browser',
    )
    await expect(
      page.getByRole('heading', { name: 'New thread' }),
    ).toBeVisible()
    await expect(page.locator('.traffic, .tl-close')).toHaveCount(0)

    await page.getByRole('button', { name: 'Settings' }).click()
    await expect(page.getByRole('heading', { name: 'Agents' })).toBeVisible()
    await expect(page.getByText('Claude Code')).toBeVisible()
    await expect(page.getByText('Cursor CLI')).toHaveCount(0)
    await expect(page.getByText('Codex CLI')).toHaveCount(0)
    await expect(page.getByText('Gemini CLI')).toHaveCount(0)
    await expect(page.getByText('Edits files directly')).toHaveCount(0)
    await expect(
      page.getByRole('switch', { name: 'Run everything' }),
    ).toBeVisible()
    await expect(page.getByText('Run gh auth login')).toBeVisible()
    mkdirSync('e2e/output', { recursive: true })
    await page.screenshot({
      path: 'e2e/output/agents-settings.png',
      fullPage: true,
    })
    await page.getByRole('button', { name: /^OAuth login,/ }).click()
    await expect(
      page.getByRole('heading', { name: 'OAuth login' }),
    ).toBeVisible()
    const composer = page.getByRole('textbox', { name: 'Message the agent' })
    await composer.fill('hello from the harness')
    await page.getByRole('button', { name: 'Send to agent' }).click()
    await expect(composer).toHaveValue('')
    const bubble = page.getByText('hello from the harness', { exact: true })
    await expect(bubble).toBeVisible()
    const you = page.locator('#timeline').getByText(/^You · \d{1,2}:\d{2}/)
    await expect(you).toBeVisible()
    const bubbleBox = await bubble.boundingBox()
    const youBox = await you.boundingBox()
    expect(youBox!.y).toBeGreaterThan(bubbleBox!.y)
    // What the agent is doing floats just above the prompt bar.
    const status = page.locator('[data-od-id="live-status"]')
    await expect(status).toContainText('Working')
    await page.getByRole('button', { name: 'Stop', exact: true }).click()
    await expect(status).toHaveCount(0)
    mkdirSync('e2e/output', { recursive: true })
    await page.screenshot({
      path: 'e2e/output/workspace.png',
      fullPage: true,
    })
    await page.keyboard.press('ControlOrMeta+n')
    await expect(
      page.getByRole('heading', { name: 'New thread' }),
    ).toBeVisible()
    await expect(page.locator('#new-thread-repo')).toBeVisible()
  })
})
