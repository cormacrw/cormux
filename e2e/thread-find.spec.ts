import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

const chunk = (seq: number, role: string, text: string) => ({
  id: 9000 + seq,
  threadId: 'th-lead',
  seq,
  kind: 'message',
  payload: JSON.stringify({ type: 'messageChunk', role, text }),
  createdAt: '2026-10-19 12:00:00',
})

test('⌘F finds text in the thread, steps through matches and opens a collapsed thought', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.addInitScript(
    (timeline) => {
      ;(window as { __HARNESS_TIMELINE__?: unknown }).__HARNESS_TIMELINE__ =
        timeline
    },
    [
      chunk(1, 'user', 'Why does the webhook retry forever?'),
      chunk(2, 'thought', 'The retry loop has no ceiling on attempts.'),
      chunk(3, 'user', 'Cap the retry at three attempts.'),
      chunk(4, 'agent', 'Done: it gives up after three.'),
    ],
  )
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  const composer = page.locator('#composer-input')
  await composer.click()

  await page.keyboard.press('ControlOrMeta+f')
  const input = page.getByRole('textbox', { name: 'Find in conversation' })
  await expect(input).toBeFocused()
  await input.fill('retry')
  const count = page.locator('[data-od-id="thread-find-count"]')
  // Three rows say "retry"; it starts at the newest.
  await expect(count).toHaveText('3 of 3')

  // The thought is collapsed until a match in it is the current one.
  const thought = page.locator('details', { hasText: 'no ceiling' })
  await expect(thought).not.toHaveAttribute('open', '')
  await input.press('Shift+Enter')
  await expect(count).toHaveText('2 of 3')
  await expect(thought).toHaveAttribute('open', '')
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/thread-find.png' })

  await input.press('Enter')
  await input.press('Enter')
  await expect(count).toHaveText('1 of 3')

  await input.fill('nothing like this')
  await expect(count).toHaveText('No matches')

  // Esc closes it and puts focus back where it was.
  await input.press('Escape')
  await expect(page.locator('[data-od-id="thread-find"]')).toHaveCount(0)
  await expect(composer).toBeFocused()
  expect(errors).toEqual([])
})

test('⌘F finds rows a long thread hasn’t rendered yet', async ({ page }) => {
  await page.goto('/?longThread=200')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(page.getByText('Answer 200', { exact: true })).toBeVisible()
  // The first answers are far above and not rendered.
  await expect(
    page.locator('[data-row-key]', { hasText: 'Answer 3' }),
  ).toHaveCount(0)

  await page.keyboard.press('ControlOrMeta+f')
  await page
    .getByRole('textbox', { name: 'Find in conversation' })
    .fill('Answer 3')
  // "Answer 3", "Answer 30"–"Answer 39" and "Answer 300"+ don't exist past 200, so 11 matches.
  await expect(page.locator('[data-od-id="thread-find-count"]')).toHaveText(
    '11 of 11',
  )
  await page.keyboard.press('Enter')
  await expect(page.locator('[data-od-id="thread-find-count"]')).toHaveText(
    '1 of 11',
  )
  await expect(page.getByText('Answer 3', { exact: true })).toBeInViewport()
})
