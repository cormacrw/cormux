import { expect, test } from '@playwright/test'

test('copying a message swaps the icon to a check, then back', async ({
  page,
  context,
}) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write'])
  await page.goto('/?longThread=1')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  const copy = page
    .locator('#timeline')
    .getByRole('button', { name: 'Copy message' })
    .first()
  await copy.hover()
  await expect(copy.locator('svg')).toHaveClass(/lucide-copy/)
  await copy.click()
  await expect(copy.locator('svg')).toHaveClass(/lucide-check/)
  expect(await page.evaluate(() => navigator.clipboard.readText())).not.toBe('')
  await expect(copy.locator('svg')).toHaveClass(/lucide-copy/, {
    timeout: 3000,
  })
})

test('a code block copies just its code from the corner button', async ({
  page,
  context,
}) => {
  await context.grantPermissions(['clipboard-read', 'clipboard-write'])
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
      chunk(1, 'user', 'How do I install?'),
      chunk(
        2,
        'agent',
        'Run this:\n\n```sh\npnpm install\npnpm tauri dev\n```',
      ),
    ],
  )
  await page.goto('/')
  await page
    .getByRole('button', { name: /OAuth login/ })
    .first()
    .click()
  const pre = page.locator('#timeline pre')
  const copy = pre.getByRole('button', { name: 'Copy code' })
  await pre.hover()
  await expect(copy).toBeVisible()
  await copy.click()
  await expect(copy.locator('svg')).toHaveClass(/lucide-check/)
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
    'pnpm install\npnpm tauri dev\n',
  )
  await page.screenshot({ path: 'e2e/output/code-copy.png' })
})

test('fenced code is syntax highlighted, and only highlight classes survive', async ({
  page,
}) => {
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
      chunk(1, 'user', 'Show me'),
      chunk(
        2,
        'agent',
        'Like this:\n\n```ts\n// greet\nexport function greet(name: string) {\n  return `hi ${name}`\n}\n```\n\n```\nplain text\n```\n\n<span class="fixed inset-0 hljs-string">x</span>',
      ),
    ],
  )
  await page.goto('/')
  await page
    .getByRole('button', { name: /OAuth login/ })
    .first()
    .click()
  const timeline = page.locator('#timeline')
  const ts = timeline.locator('pre code.language-ts')
  await expect(ts.locator('.hljs-keyword').first()).toHaveText('export')
  await expect(ts.locator('.hljs-comment')).toHaveText('// greet')
  const keyword = await ts
    .locator('.hljs-keyword')
    .first()
    .evaluate((el) => getComputedStyle(el).color)
  const plain = await ts.evaluate((el) => getComputedStyle(el).color)
  expect(keyword).not.toBe(plain)
  await expect(timeline.locator('pre').nth(1).locator('span')).toHaveCount(0)
  await expect(
    timeline.locator('span.hljs-string', { hasText: 'x' }),
  ).toHaveAttribute('class', 'hljs-string')
  await page.screenshot({ path: 'e2e/output/code-highlight.png' })
})
