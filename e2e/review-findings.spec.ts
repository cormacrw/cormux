import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

const finding = (
  id: string,
  severity: string,
  title: string,
  file: string | null,
  line: number | null,
  explanation: string,
) => ({
  id,
  workspaceId: 'ws-auth',
  severity,
  title,
  file,
  line,
  explanation,
  status: 'open',
  commitSha: null,
  sentToThreadId: null,
})

test('findings: selectable text, checkbox-only toggles, location opens Changes at the line', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.addInitScript(
    (findings) => {
      ;(
        window as { __HARNESS_REVIEW_FINDINGS__?: unknown }
      ).__HARNESS_REVIEW_FINDINGS__ = findings
    },
    [
      finding(
        'f1',
        'blocking',
        'GitHub sessions skip the expiry check',
        'src/auth/session.ts',
        4,
        'getSession returns the raw oauth cookie for GitHub, so an expired token still counts as signed in. Validate the expiry before returning it.',
      ),
      finding(
        'f2',
        'suggestion',
        'Add a test for the provider branch',
        'src/auth/session.ts',
        null,
        'Nothing covers getSession("github").',
      ),
      finding(
        'f3',
        'nit',
        'Tidy the import order',
        null,
        null,
        'Group local imports after packages.',
      ),
    ],
  )
  await page.goto('/')
  await page
    .getByRole('button', { name: /OAuth login/ })
    .first()
    .click()
  // The thread ends with a slim findings row that opens the tab.
  const findingsRow = page.locator('[data-od-id="findings-card"]')
  await expect(findingsRow).toContainText(
    'Review findings · 1 blocking, 1 suggestion, 1 nit',
  )
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/review-findings-thread.png' })
  await findingsRow.getByRole('button', { name: /Open findings/ }).click()

  const panel = page.locator('#findings-panel')
  await expect(
    panel.getByRole('heading', { name: 'Review findings' }),
  ).toBeVisible()
  await expect(panel.getByText('Known gaps')).toHaveCount(0)

  // Blocking is selected by default; clicking the text doesn't toggle it.
  const blocking = panel.getByRole('checkbox', {
    name: 'Select “GitHub sessions skip the expiry check”',
  })
  await expect(blocking).toBeChecked()
  await panel.getByText('getSession returns the raw oauth cookie').click()
  await expect(blocking).toBeChecked()

  // The text can be selected.
  const selected = await panel
    .getByText('getSession returns the raw oauth cookie')
    .evaluate((el) => {
      const range = document.createRange()
      range.selectNodeContents(el)
      const selection = window.getSelection()!
      selection.removeAllRanges()
      selection.addRange(range)
      return selection.toString()
    })
  expect(selected).toContain('expired token')
  expect(
    await panel
      .getByText('getSession returns the raw oauth cookie')
      .evaluate((el) => getComputedStyle(el).userSelect),
  ).not.toBe('none')

  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/review-findings.png' })

  await panel.getByRole('button', { name: 'src/auth/session.ts:4' }).click()
  await expect(page.locator('#changes-panel')).toBeVisible()
  const row = page
    .locator('[data-diff-path="src/auth/session.ts"] [data-line-new-num="4"]')
    .first()
  await expect(row).toBeInViewport()
  await page.screenshot({ path: 'e2e/output/review-findings-changes.png' })
  expect(errors).toEqual([])
})

test('the Reviewer’s findings block is hidden from the conversation', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  const chunk = (seq: number, role: string, text: string) => ({
    id: 9000 + seq,
    threadId: 'th-lead',
    seq,
    kind: 'message',
    payload: JSON.stringify({ type: 'messageChunk', role, text }),
    createdAt: '2026-09-29 12:00:00',
  })
  await page.addInitScript(
    (timeline) => {
      ;(window as { __HARNESS_TIMELINE__?: unknown }).__HARNESS_TIMELINE__ =
        timeline
    },
    [
      chunk(1, 'user', 'Review #2'),
      chunk(
        2,
        'agent',
        'One blocking issue in the session code.\n\n<cormux-fin',
      ),
      chunk(
        3,
        'agent',
        'dings>\n[{"severity":"blocking","title":"Expiry skipped"}]\n</cormux-findings>',
      ),
    ],
  )
  await page.goto('/')
  await page
    .getByRole('button', { name: /OAuth login/ })
    .first()
    .click()
  const timeline = page.locator('#timeline')
  await expect(
    timeline.getByText('One blocking issue in the session code.'),
  ).toBeVisible()
  await expect(timeline.getByText('cormux-findings')).toHaveCount(0)
  await expect(timeline.getByText('Expiry skipped')).toHaveCount(0)
  expect(errors).toEqual([])
})
