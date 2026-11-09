import { expect, test, type Page } from '@playwright/test'
import type { FindingRow } from '../src/lib/ipc/bindings'
import { setScenario } from './harness'

// State that belongs to one thread or workspace stays with it, and a reply lands on the one it
// was asked for.

const finding = (
  id: string,
  workspaceId: string,
  title: string,
  status: FindingRow['status'] = 'open',
): FindingRow => ({
  id,
  workspaceId,
  severity: 'blocking',
  title,
  file: null,
  line: null,
  explanation: 'Because.',
  status,
  commitSha: null,
  sentToThreadId: null,
})

async function runCommand(page: Page, label: string) {
  await page.getByRole('button', { name: /Search or run a command/ }).click()
  const dialog = page.getByRole('dialog')
  await page.keyboard.type(label)
  await dialog.getByText(label, { exact: true }).click()
  await expect(dialog).toHaveCount(0)
}

function toast(page: Page, text: string | RegExp) {
  return page.locator('[data-sonner-toast]', { hasText: text })
}

test('Findings opened for another workspace over open Findings shows its panel', async ({
  page,
}) => {
  await setScenario(page, {
    secondWorkspace: true,
    reviewFindings: [
      finding('f1', 'ws-auth', 'Sessions skip the expiry check'),
      finding('f2', 'ws-billing', 'Invoices round down'),
    ],
  })
  await page.goto('/')
  await runCommand(page, 'Open Findings in OAuth login')
  const panel = page.locator('#findings-panel')
  await expect(panel).toContainText('Sessions skip the expiry check')

  await runCommand(page, 'Open Findings in Billing fix')
  await expect(
    page.locator('[data-od-id="findings-slide-out"]'),
  ).toHaveAttribute('data-open', 'true')
  await expect(panel).toBeVisible()
  await expect(panel).toContainText('Invoices round down')
})

test('a send still under way in one thread doesn’t hold up another’s composer', async ({
  page,
}) => {
  await setScenario(page, {
    secondWorkspace: true,
    slow: { send_thread_prompt: 10_000 },
  })
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  const composer = page.locator('#composer-input')
  const send = page.getByRole('button', { name: 'Send to agent' })
  await composer.fill('Take a while')
  await send.click()
  await expect(send).toHaveAttribute('aria-busy', 'true')

  await page.getByRole('button', { name: /^Billing fix,/ }).click()
  await expect(page.getByRole('heading', { name: 'Billing fix' })).toBeVisible()
  await composer.fill('Go now')
  await expect(send).toHaveAttribute('aria-busy', 'false')
  await expect(send).toBeEnabled()
})

test('the model picker shows nothing of the last thread’s pick while the next one’s loads', async ({
  page,
}) => {
  await setScenario(page, {
    secondWorkspace: true,
    slow: { thread_models: 1000 },
  })
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  const model = page.locator('[data-od-id="composer-model"]')
  await model.click()
  await page.getByRole('menuitemradio', { name: 'Opus' }).click()
  await expect(model).toHaveText('Opus')

  await page.getByRole('button', { name: /^Billing fix,/ }).click()
  await expect(model).toHaveCount(0)
  await expect(model).toHaveText('Default')
})

test('a model list that can’t be read says so', async ({ page }) => {
  await setScenario(page, { failing: { thread_models: 'No models today' } })
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(toast(page, 'No models today').first()).toBeVisible()
})

test('a deny that fails keeps the reason typed for it', async ({ page }) => {
  await setScenario(page, {
    failing: { resolve_approval: 'Agent went away' },
    timeline: [
      {
        id: 9101,
        threadId: 'th-lead',
        seq: 1,
        kind: 'permission',
        payload: JSON.stringify({
          type: 'permission',
          id: 'a1',
          tool_call_id: 'toolu_bash',
          card: {
            title: 'Use Bash',
            what: 'pnpm test',
            why: 'Bash',
            okLabel: 'Approve',
            noLabel: 'Deny',
          },
          tool: 'Bash',
          role: { kind: 'execute' },
          detail: 'pnpm test',
          auto_approved: false,
        }),
        createdAt: '2026-10-30 12:00:00',
      },
    ],
    approvals: [
      {
        id: 'a1',
        threadId: 'th-lead',
        status: 'pending',
        tool: 'execute',
        payload: {
          title: 'Run a command',
          what: 'pnpm test',
          why: 'Bash',
          okLabel: 'Approve',
          noLabel: 'Deny',
        },
      },
    ],
  })
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  const card = page.locator('[data-od-id="approval-a1"]')
  await card.getByRole('button', { name: 'Deny' }).click()
  const reason = card.getByPlaceholder('Tell the agent why (optional)')
  await reason.fill('Not on main')
  await card.getByRole('button', { name: 'Deny' }).click()
  await expect(toast(page, 'Agent went away')).toBeVisible()
  await expect(reason).toHaveValue('Not on main')
})

test('a setting the core refuses to save goes back', async ({ page }) => {
  await setScenario(page, { failing: { update_settings: 'Disk full' } })
  await page.goto('/')
  await page.getByRole('button', { name: 'Settings' }).click()
  const toggle = page.getByRole('switch', { name: 'Run everything' })
  const before = await toggle.getAttribute('aria-checked')
  await toggle.click()
  await expect(toast(page, 'Disk full')).toBeVisible()
  await expect(toggle).toHaveAttribute('aria-checked', before!)
})

test('a failed Restore says why, with every finding dismissed', async ({
  page,
}) => {
  await setScenario(page, {
    reviewFindings: [finding('f1', 'ws-auth', 'Old news', 'dismissed')],
    failing: { dismiss_workspace_findings: 'Store locked' },
  })
  await page.goto('/')
  await runCommand(page, 'Open Findings in OAuth login')
  const panel = page.locator('#findings-panel')
  await panel.getByRole('button', { name: 'Restore “Old news”' }).click()
  await expect(panel.getByText('Store locked')).toBeVisible()
})

test('a task whose description ClickUp won’t give still goes, and you’re told', async ({
  page,
}) => {
  await setScenario(page, {
    clickup: true,
    failing: { clickup_task: 'ClickUp is down' },
  })
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  const input = page.locator('#composer-input')
  await input.click()
  await input.pressSequentially('Fix #eng-198')
  await input.press('Escape')
  await input.press('Enter')
  await expect(
    toast(page, /Sent without the description of “Session expires/),
  ).toBeVisible()
  await expect
    .poll(async () => {
      const prompts = await page.evaluate(() => window.__HARNESS__!.prompts)
      return prompts?.at(-1)?.text ?? ''
    })
    .toMatch(/^Fix ENG-198 “Session expires mid checkout on Safari”/)
})

test('a turn that ended off screen takes its tool out of the live status', async ({
  page,
}) => {
  await setScenario(page, { secondWorkspace: true })
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await page.evaluate(() => {
    window.__HARNESS__!.setThreadStatus('th-lead', 'running')
    window.__HARNESS__!.agentEvents([
      { type: 'currentTool', id: 'tool-1', title: 'Reading session.ts' },
    ])
  })
  const status = page.locator('[data-od-id="live-status"]')
  await expect(status).toContainText('Reading session.ts')

  await page.getByRole('button', { name: /^Billing fix,/ }).click()
  await expect(page.getByRole('heading', { name: 'Billing fix' })).toBeVisible()
  await page.evaluate(() => {
    window.__HARNESS__!.agentEvents([
      { type: 'turnEnd', stop_reason: 'end_turn', error: null },
    ])
    window.__HARNESS__!.setThreadStatus('th-lead', 'idle')
  })
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(page.locator('#composer-input')).toBeVisible()
  await page.waitForTimeout(300)

  await page.evaluate(() =>
    window.__HARNESS__!.setThreadStatus('th-lead', 'running'),
  )
  await expect(status).toBeVisible()
  await expect(status).not.toContainText('Reading session.ts')
})

test('a push under way in one workspace’s stack leaves another’s buttons alone', async ({
  page,
}) => {
  await setScenario(page, {
    secondWorkspace: true,
    slow: { push_stack: 10_000 },
  })
  await page.setViewportSize({ width: 1400, height: 900 })
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  // Pinned, the tray stays up from one workspace to the next.
  await page.keyboard.press('ControlOrMeta+g')
  await page
    .getByRole('button', { name: 'Pin Git beside the conversation' })
    .click()
  const stack = page
    .getByRole('region', { name: 'Git', exact: true })
    .getByRole('navigation', { name: 'Stack' })
  const push = stack.getByRole('button', { name: 'Push' })
  await push.click()
  await expect(push).toBeDisabled()

  await page.getByRole('button', { name: /^Billing fix,/ }).click()
  await expect(stack).toContainText('fix/billing')
  await expect(stack.getByRole('button', { name: 'Add branch' })).toBeEnabled()
})

test('Esc still closes a tray after the ⋯ menu went away while open', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await page.getByRole('button', { name: 'More workspace actions' }).click()
  await expect(page.getByRole('menu')).toBeVisible()
  // As the native menu can: the workspace view goes away with the menu still open.
  await page.evaluate(async (url) => {
    const nav = (await import(url)) as {
      navigate: (route: { view: 'newThread' }) => void
    }
    nav.navigate({ view: 'newThread' })
  }, '/src/lib/core/nav.svelte.ts')
  await expect(page.locator('[data-od-id="new-thread"]')).toBeVisible()

  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await page.locator('#tray-changes').click()
  const tray = page.locator('[data-od-id="git-slide-out"]')
  await expect(tray).toHaveAttribute('data-open', 'true')
  await page.keyboard.press('Escape')
  await expect(tray).toHaveAttribute('data-open', 'false')
})

test('Add run command lands in the repo’s run command field and stays there', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await page.keyboard.press('ControlOrMeta+o')
  await page.getByRole('button', { name: 'Add run command' }).click()
  const field = page.locator('#run-my-app')
  await expect(field).toBeFocused()
  await page.waitForTimeout(800)
  await expect(field).toBeFocused()
  await expect(field).toBeInViewport()
})
