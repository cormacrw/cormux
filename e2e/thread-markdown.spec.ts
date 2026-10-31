import { expect, test, type Page } from '@playwright/test'

async function openReply(page: Page, reply: string) {
  const chunk = (seq: number, role: string, text: string) => ({
    id: 9100 + seq,
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
    [chunk(1, 'user', 'Compare the options'), chunk(2, 'agent', reply)],
  )
  await page.goto('/')
  await page
    .getByRole('button', { name: /OAuth login/ })
    .first()
    .click()
}

test('a table in a reply renders as a table', async ({ page }) => {
  await openReply(
    page,
    [
      'Here is how they compare:',
      '',
      '| Option | Cost | Notes |',
      '|:-------|:----:|------:|',
      '| `marked` | **free** | [docs](https://marked.js.org) |',
      '| Longhand | 112 kB | Written by hand over a long and rainy weekend, with several false starts along the way, a good deal of tea, and one rewrite nobody asked for |',
    ].join('\n'),
  )

  const table = page.locator('#timeline .agent-markdown table')
  await expect(table.getByRole('columnheader')).toHaveText([
    'Option',
    'Cost',
    'Notes',
  ])
  await expect(table.getByRole('row')).toHaveCount(3)
  await expect(table.locator('td strong')).toHaveText('free')
  await expect(table.getByRole('link', { name: 'docs' })).toBeVisible()

  // The column's alignment holds, and words wrap rather than letters.
  await expect(table.getByRole('cell', { name: 'free' })).toHaveCSS(
    'text-align',
    'center',
  )
  await expect(table.getByRole('cell', { name: '112 kB' })).toHaveCSS(
    'text-align',
    'center',
  )
  const lineCount = (cell: HTMLElement) => {
    const range = document.createRange()
    range.selectNodeContents(cell)
    return range.getClientRects().length
  }
  await expect
    .poll(() => table.getByRole('cell', { name: '112 kB' }).evaluate(lineCount))
    .toBe(1)
  await expect
    .poll(() =>
      table.getByRole('cell', { name: /^Written/ }).evaluate(lineCount),
    )
    .toBeGreaterThan(1)

  await page.screenshot({ path: 'e2e/output/thread-table.png' })
})

test('a table too wide for the thread scrolls by itself', async ({ page }) => {
  const wide = Array.from({ length: 8 }, (_, at) => `columnheading${at}`)
  await openReply(
    page,
    [
      `| ${wide.join(' | ')} |`,
      `|${wide.map(() => '---').join('|')}|`,
      `| ${wide.map(() => 'value').join(' | ')} |`,
    ].join('\n'),
  )

  const table = page.locator('#timeline .agent-markdown table')
  await expect(table).toBeVisible()
  const { scrollable, parentOverflow } = await table.evaluate((element) => ({
    scrollable: element.scrollWidth > element.clientWidth,
    parentOverflow:
      element.parentElement!.scrollWidth > element.parentElement!.clientWidth,
  }))
  expect(scrollable).toBe(true)
  expect(parentOverflow).toBe(false)
  await page.screenshot({ path: 'e2e/output/thread-table-wide.png' })
})

const fence = (lang: string, code: string) => `\`\`\`${lang}\n${code}\n\`\`\``

test('a reply’s rich blocks render as components', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 2000 })
  await openReply(
    page,
    [
      'See src/lib/thread/typing.ts:40 and `src/app.css`, costing $5 to $10, where $x^2$ grows.',
      '',
      '> [!WARNING]',
      '> This migration can’t be undone.',
      '',
      fence(
        'diff',
        '--- a/src/a.ts\n+++ b/src/a.ts\n@@ -1,2 +1,2 @@\n-const a = 1\n+const a = 2\n same()',
      ),
      '',
      fence(
        '',
        'FAIL src/a.test.ts > adds\n     Tests  1 failed | 268 passed (269)',
      ),
      '',
      fence('mermaid', 'graph TD;\n  A[Webview]-->B[Core];'),
      '',
      fence(
        'chart',
        JSON.stringify({
          type: 'bar',
          title: 'Bundle size (kB)',
          labels: ['main', 'vendor', 'mermaid'],
          series: [
            { name: 'Before', values: [120, 340, 0] },
            { name: 'After', values: [124, 338, 610] },
          ],
        }),
      ),
      '',
      fence(
        'json',
        JSON.stringify(
          {
            name: 'cormux',
            nested: { a: 1, b: [1, 2, 3, 4, 5], c: { d: true } },
          },
          null,
          8,
        ),
      ),
      '',
      '$$\\int_0^1 x^2 dx$$',
      '',
      'Merged in [the PR](https://github.com/acme/my-app/pull/42).',
    ].join('\n'),
  )
  const reply = page
    .locator('#timeline .agent-markdown:not(.agent-markdown .agent-markdown)')
    .last()

  await expect(reply.locator('.file-ref')).toHaveCount(2)
  await expect(reply.locator('.file-ref').first()).toHaveText(
    'src/lib/thread/typing.ts:40',
  )
  await expect(reply.getByText('costing $5 to $10')).toBeVisible()
  await expect(reply.locator('p .katex')).toBeVisible()

  await expect(reply.locator('.callout-warning')).toContainText('Warning')

  const diff = reply.locator('[data-od-id="reply-diff"]')
  await expect(diff).toContainText('src/a.ts')
  await expect(diff.locator('[data-line="add"]')).toHaveText('+const a = 2')

  const run = reply.locator('[data-od-id="reply-run"]')
  await expect(run).toContainText('268 passed, 1 failed')
  await expect(reply.locator('.reply-run-failures')).toContainText(
    'src/a.test.ts > adds',
  )

  await expect(reply.locator('[data-od-id="reply-mermaid"] svg')).toContainText(
    'Webview',
  )

  const chart = reply.locator('[data-od-id="reply-chart"]')
  await expect(chart).toContainText('Bundle size (kB)')
  await chart.locator('[data-chart-slot="2"]').hover()
  await expect(chart.getByRole('status')).toContainText('610')
  await chart.getByRole('button', { name: 'Table' }).click()
  await expect(chart.getByRole('cell', { name: '338' })).toBeVisible()
  await chart.getByRole('button', { name: 'Chart' }).click()

  // The reply sanitiser still strips app classes from what an agent writes.
  await expect(reply.locator('p .bg-primary')).toHaveCount(0)
  await expect(reply.locator('[data-od-id="reply-json"]')).toContainText(
    'cormux',
  )
  await expect(reply.locator('[data-od-id="reply-math"] .katex')).toBeVisible()
  await expect(reply.locator('a.ref-card').first()).toContainText(
    'acme/my-app#42',
  )

  await page.screenshot({
    path: 'e2e/output/thread-blocks.png',
    fullPage: true,
  })
  await reply.locator('.file-ref').first().click()
  await expect(
    page
      .getByRole('heading', { name: 'Changes' })
      .or(page.locator('[data-od-id="changes-panel"]'))
      .first(),
  ).toBeVisible()
})

test('a long tool run folds, and plans and subagents show progress', async ({
  page,
}) => {
  const event = (seq: number, payload: object, kind = 'tool') => ({
    id: 9500 + seq,
    threadId: 'th-lead',
    seq,
    kind,
    payload: JSON.stringify(payload),
    createdAt: '2026-09-29 12:00:00',
  })
  const tool = (id: string, kind: string, title: string, extra = {}) => ({
    type: 'toolCall',
    id,
    title,
    name: title,
    kind,
    status: 'completed',
    locations: [],
    detail: null,
    parent_id: null,
    ...extra,
  })
  const events = [
    { type: 'messageChunk', role: 'user', text: 'Do the work' },
    {
      type: 'plan',
      entries: [
        { content: 'Read the code', status: 'pending' },
        { content: 'Fix it', status: 'pending' },
      ],
    },
    {
      type: 'plan',
      entries: [
        { content: 'Read the code', status: 'completed' },
        { content: 'Fix it', status: 'in_progress' },
      ],
    },
    tool('r1', 'read', 'Read', { locations: ['a.ts'] }),
    tool('r2', 'read', 'Read', { locations: ['b.ts'] }),
    tool('r3', 'read', 'Read', { locations: ['c.ts'] }),
    tool('e1', 'edit', 'Edit', { locations: ['src/a.ts'] }),
    tool('c1', 'execute', 'pnpm test'),
    {
      type: 'toolCallUpdate',
      id: 'c1',
      title: null,
      kind: null,
      status: 'completed',
      locations: [],
      output: '  Tests  12 passed (12)\n',
    },
    tool('a1', 'other', 'Agent', { detail: 'Explore the code' }),
    tool('s1', 'read', 'Read', { parent_id: 'a1', locations: ['d.ts'] }),
    tool('s2', 'execute', 'ls', { parent_id: 'a1', status: 'in_progress' }),
    { type: 'messageChunk', role: 'agent', text: 'All done.' },
  ]
  await page.addInitScript(
    (timeline) => {
      ;(window as { __HARNESS_TIMELINE__?: unknown }).__HARNESS_TIMELINE__ =
        timeline
    },
    events.map((payload, at) =>
      event(
        at + 1,
        payload,
        payload.type === 'messageChunk' ? 'message' : 'tool',
      ),
    ),
  )
  await page.goto('/')
  await page
    .getByRole('button', { name: /OAuth login/ })
    .first()
    .click()

  const timeline = page.locator('#timeline')
  await expect(timeline.getByText('Plan · 1 of 2')).toBeVisible()
  await expect(timeline.getByText('Proposed plan')).toHaveCount(0)
  await expect(
    timeline.locator('[data-od-id="plan-steps"] [data-status="completed"]'),
  ).toHaveText('Read the code')

  const run = timeline.locator('[data-od-id="tool-run-summary"]')
  await expect(run.locator('summary').first()).toContainText(
    'Read 3 files, edited 1 file, ran 1 command',
  )
  await expect(timeline.locator('[data-od-id="files-touched"]')).toContainText(
    'a.ts',
  )
  await run.locator('summary').first().click()
  await expect(run.locator('[data-od-id="reply-run"]')).toContainText(
    '12 passed',
  )
  const subagent = run.locator('[data-od-id="subagent-steps"]')
  await expect(subagent.getByRole('listitem')).toHaveCount(2)
  await expect(subagent.getByRole('img', { name: 'in_progress' })).toHaveCount(
    1,
  )
  await page.screenshot({ path: 'e2e/output/thread-run.png' })
})

test('before and after screenshots compare with a slider', async ({ page }) => {
  await openReply(
    page,
    '![Before](/tmp/shots/before.png)\n\n![After the fix](/tmp/shots/after.png)',
  )
  const compare = page.locator('[data-od-id="reply-compare"]')
  await expect(compare.getByRole('img', { name: 'Before' })).toBeVisible()
  await compare.getByRole('slider').fill('30')
  await expect(compare.getByRole('img', { name: 'Before' })).toHaveCSS(
    'clip-path',
    /70%/,
  )
})
