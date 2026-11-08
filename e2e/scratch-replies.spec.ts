import { expect, test } from '@playwright/test'
import { setScenario } from './harness'

const reply = (threadId: string, id: number, seq: number, text: string) => ({
  id,
  threadId,
  seq,
  kind: 'message',
  payload: JSON.stringify({ type: 'messageChunk', role: 'agent', text }),
  createdAt: '2026-09-29 12:00:00',
})

const TEXT =
  'The bug is in src/lib/stripe.ts:12, see `src/app.css` and [the helper](src/lib/stripe.ts).'

test('file references are links in a workspace thread, and plain text in a scratch, which has no Changes', async ({
  page,
}) => {
  await setScenario(page, {
    timeline: [
      reply('th-lead', 9201, 1, TEXT),
      reply('th-scratch-webhook', 9202, 99, TEXT),
    ],
  })
  await page.goto('/')
  const sidebar = page.getByRole('navigation', { name: 'Sidebar' })
  const lastReply = page
    .locator('#timeline .agent-markdown:not(.agent-markdown .agent-markdown)')
    .last()

  await sidebar.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(lastReply.locator('.file-ref')).toHaveCount(2)
  await expect(lastReply.locator('.file-ref-link')).toHaveCount(1)

  await sidebar
    .getByRole('button', { name: /Why the webhook signature fails/ })
    .click()
  await expect(lastReply).toContainText('src/lib/stripe.ts:12')
  await expect(lastReply.locator('.file-ref, .file-ref-link')).toHaveCount(0)
  await expect(lastReply.locator('a[href]')).toHaveCount(0)
})
