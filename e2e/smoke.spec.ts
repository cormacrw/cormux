import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test.describe('browser harness UI', () => {
  test('opens New workspace and keeps the footer on the dialog', async ({
    page,
  }) => {
    await page.goto('/')
    await expect(page.locator('html')).toHaveAttribute(
      'data-cormux-harness',
      'browser',
    )
    await expect(page.getByRole('heading', { name: 'Homebase' })).toBeVisible()
    await expect(page.locator('.traffic, .tl-close')).toHaveCount(0)

    await page.getByRole('button', { name: 'Settings' }).click()
    await expect(page.getByRole('heading', { name: 'Agents' })).toBeVisible()
    await expect(page.getByText('Claude Code')).toBeVisible()
    await expect(page.getByText('Cursor CLI')).toBeVisible()
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
    await page.getByRole('button', { name: 'Homebase' }).click()

    await page
      .getByRole('button', { name: 'OAuth login, Idle, 1 agent' })
      .click()
    await expect(
      page.getByRole('heading', { name: 'OAuth login' }),
    ).toBeVisible()
    const composer = page.getByRole('textbox', { name: 'Message the agent' })
    await composer.fill('hello from the harness')
    await page.getByRole('button', { name: 'Send to agent' }).click()
    await expect(composer).toHaveValue('')
    const bubble = page.getByText('hello from the harness')
    await expect(bubble).toBeVisible()
    const you = page.locator('#timeline').getByText(/^You · \d{1,2}:\d{2}/)
    await expect(you).toBeVisible()
    const bubbleBox = await bubble.boundingBox()
    const youBox = await you.boundingBox()
    expect(youBox!.y).toBeGreaterThan(bubbleBox!.y)
    await expect(
      page.locator('#timeline').getByText('Working').first(),
    ).toBeVisible()
    await page.getByRole('button', { name: 'Stop', exact: true }).click()
    await expect(page.locator('#timeline').getByText('Working')).toHaveCount(0)
    mkdirSync('e2e/output', { recursive: true })
    await page.screenshot({
      path: 'e2e/output/workspace.png',
      fullPage: true,
    })
    await page.getByRole('button', { name: 'Homebase' }).click()
    await expect(page.getByRole('heading', { name: 'Homebase' })).toBeVisible()
    await expect(
      page.getByRole('button', { name: 'Go to Workspace' }),
    ).toHaveCount(0)
    await page.screenshot({
      path: 'e2e/output/homebase.png',
      fullPage: true,
    })
    await page.getByRole('button', { name: 'OAuth login', exact: true }).click()
    await expect(
      page.getByRole('heading', { name: 'OAuth login' }),
    ).toBeVisible()
    await page.getByRole('button', { name: 'Homebase' }).click()

    await page
      .getByRole('button', { name: 'New Workspace', exact: true })
      .click()

    const dialog = page.getByRole('dialog', { name: 'New workspace' })
    await expect(dialog).toBeVisible()
    await expect(dialog.getByText('No repositories yet')).toHaveCount(0)
    await expect(dialog.locator('#nw-repo')).toBeVisible()
    await expect(dialog.locator('#nw-engine')).toBeVisible()
    await expect(dialog.getByText('The agent provider to use')).toBeVisible()
    await expect(
      dialog.getByText('Describe the task so the agent knows where to start.'),
    ).toHaveCount(0)
    await expect(
      dialog.getByText(
        'Optional. Leave blank and the thread waits until you send a message.',
      ),
    ).toBeVisible()

    const footerHint = dialog.getByRole('button', { name: /Cancel/ })
    await expect(footerHint).toBeVisible()

    const dialogBox = await dialog.boundingBox()
    const footerBox = await footerHint.boundingBox()
    expect(dialogBox).toBeTruthy()
    expect(footerBox).toBeTruthy()
    if (!dialogBox || !footerBox) return

    expect(footerBox.x).toBeGreaterThanOrEqual(dialogBox.x - 1)
    expect(footerBox.y).toBeGreaterThanOrEqual(dialogBox.y - 1)
    expect(footerBox.x + footerBox.width).toBeLessThanOrEqual(
      dialogBox.x + dialogBox.width + 1,
    )
    expect(footerBox.y + footerBox.height).toBeLessThanOrEqual(
      dialogBox.y + dialogBox.height + 1,
    )

    mkdirSync('e2e/output', { recursive: true })
    await page.screenshot({
      path: 'e2e/output/new-workspace.png',
      fullPage: true,
    })
  })

  test('hides the engine select when only one agent is installed', async ({
    page,
  }) => {
    await page.goto('/')
    await page.evaluate(() => {
      ;(window as { __HARNESS_ENGINES__?: unknown }).__HARNESS_ENGINES__ = [
        {
          kind: 'claude',
          installed: true,
          binary: '/usr/local/bin/claude',
          version: '1.0.0',
          signedIn: true,
        },
        {
          kind: 'cursor',
          installed: false,
          binary: null,
          version: null,
          signedIn: null,
        },
      ]
    })
    await page
      .getByRole('button', { name: 'New Workspace', exact: true })
      .click()
    const dialog = page.getByRole('dialog', { name: 'New workspace' })
    await expect(dialog).toBeVisible()
    await expect(dialog.locator('#nw-engine')).toHaveCount(0)
    await expect(dialog.getByText('The agent provider to use')).toHaveCount(0)
    await expect(dialog.locator('#nw-repo')).toBeVisible()
    await page.screenshot({ path: 'e2e/output/new-workspace-one-engine.png' })
  })
})
