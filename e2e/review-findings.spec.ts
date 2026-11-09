import { expect, test } from '@playwright/test'
import type { FindingRow, FindingSeverity } from '../src/lib/ipc/bindings'
import { setScenario } from './harness'
import { mkdirSync } from 'node:fs'

const finding = (
  id: string,
  severity: FindingSeverity,
  title: string,
  file: string | null,
  line: number | null,
  explanation: string,
): FindingRow => ({
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
  await setScenario(page, {
    reviewFindings: [
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
  })
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
    .locator('#changes-file-src_auth_session_ts [data-line-new-num="4"]')
    .first()
  await expect(row).toBeInViewport()
  // The line flashes, though the panel had only just mounted.
  await expect
    .poll(() =>
      row.evaluate((el) =>
        [...el.closest('tr')!.cells].some(
          (cell) => cell.getAnimations().length > 0,
        ),
      ),
    )
    .toBe(true)
  // Only the diff scrolled: the row the tray slides into stays put.
  expect(
    await page
      .locator('[data-od-id="git-slide-out"]')
      .evaluate((tray) => tray.parentElement!.scrollLeft),
  ).toBe(0)
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
  await setScenario(page, {
    timeline: [
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
  })
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

test('findings: dismiss one or the selection, and restore it', async ({
  page,
}) => {
  await setScenario(page, {
    reviewFindings: [
      finding('f1', 'blocking', 'Expiry not checked', 'a.ts', 4, 'Check it.'),
      finding('f2', 'suggestion', 'Add a test', null, null, 'Cover it.'),
      finding('f3', 'nit', 'Tidy imports', null, null, 'Group them.'),
    ],
  })
  await page.goto('/')
  await page
    .getByRole('button', { name: /OAuth login/ })
    .first()
    .click()
  await page.locator('#tray-findings').click()
  const panel = page.locator('#findings-panel')
  const tab = page.locator('#tray-findings')
  await expect(tab).toHaveAccessibleName(/3 open findings/)

  // One at a time: the row stays, dimmed, and can't be selected.
  await panel.getByRole('button', { name: 'Dismiss “Tidy imports”' }).click()
  const nit = panel.locator('[data-od-id="finding-f3"]')
  await expect(nit).toContainText('Dismissed')
  await expect(
    nit.getByRole('checkbox', { name: 'Select “Tidy imports”' }),
  ).toBeDisabled()
  await expect(tab).toHaveAccessibleName(/2 open findings/)

  // The selection: blocking is selected by default.
  await panel.locator('[data-od-id="findings-dismiss-selected"]').click()
  await expect(panel.locator('[data-od-id="finding-f1"]')).toContainText(
    'Dismissed',
  )
  await expect(tab).toHaveAccessibleName(/1 open finding\b/)

  await panel.getByRole('button', { name: 'Restore “Tidy imports”' }).click()
  await expect(nit).not.toContainText('Dismissed')
  await expect(tab).toHaveAccessibleName(/2 open findings/)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/findings-dismissed.png' })
})

test('a PR review that found nothing opens Findings on “Nothing to fix”', async ({
  page,
}) => {
  await setScenario(page, { reviewFindings: [] })
  await page.goto('/')
  await page
    .getByRole('button', { name: /OAuth login/ })
    .first()
    .click()
  await page.locator('#tray-findings').click()
  await expect(page.locator('[data-od-id="findings-empty"]')).toContainText(
    'Nothing to fix',
  )
  await expect(page.locator('#tray-findings')).toHaveAttribute(
    'aria-pressed',
    'true',
  )
})

test('findings: picking findings keeps the list where it was scrolled', async ({
  page,
}) => {
  await setScenario(page, {
    reviewFindings: Array.from({ length: 24 }, (_, index) =>
      finding(
        `f${index}`,
        index % 2 ? 'suggestion' : 'blocking',
        `Finding ${index}`,
        null,
        null,
        'A paragraph of explanation that takes up a little room in the list.',
      ),
    ),
  })
  await page.goto('/')
  await page
    .getByRole('button', { name: /OAuth login/ })
    .first()
    .click()
  await page.locator('#tray-findings').click()
  const panel = page.locator('#findings-panel')
  const box = panel.getByRole('checkbox', { name: 'Select “Finding 21”' })
  await box.scrollIntoViewIfNeeded()
  const scroller = panel.locator('.overflow-y-auto').first()
  const top = await scroller.evaluate((el) => el.scrollTop)
  expect(top).toBeGreaterThan(0)
  await box.click()
  await expect(box).toBeChecked()
  // Dispatched, since a real click would first scroll the header's None into view.
  await panel
    .getByRole('button', { name: 'None', exact: true })
    .dispatchEvent('click')
  await expect(box).not.toBeChecked()
  expect(await scroller.evaluate((el) => el.scrollTop)).toBe(top)
})
