import { expect, test } from '@playwright/test'

test('media an agent references in a reply renders inline', async ({
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
      chunk(1, 'user', 'Show me the new settings page'),
      chunk(
        2,
        'agent',
        [
          'Here it is after the fix:',
          '',
          '![Settings after the fix](/tmp/shots/settings-after.png)',
          '',
          '[Watch the flow](/tmp/shots/onboarding.webm)',
          '',
          'The toggle now stays put.',
        ].join('\n'),
      ),
    ],
  )
  await page.goto('/')
  await page
    .getByRole('button', { name: /OAuth login/ })
    .first()
    .click()

  const timeline = page.locator('#timeline')
  const image = timeline.getByRole('img', {
    name: 'Settings after the fix',
  })
  await expect(image).toBeVisible()
  await expect(
    timeline.getByText('Settings after the fix', { exact: true }),
  ).toBeVisible()
  await expect(timeline.locator('video')).toHaveCount(1)
  await expect(timeline.getByText('The toggle now stays put.')).toBeVisible()
  // The raw markdown reference is not left in the prose.
  await expect(timeline.getByText(/!\[Settings after the fix\]/)).toHaveCount(0)

  await timeline
    .getByRole('button', { name: 'Enlarge settings-after.png' })
    .click()
  await expect(
    page.getByRole('dialog', { name: 'settings-after.png' }),
  ).toBeVisible()
  await page.getByRole('button', { name: 'Close' }).click()
  await expect(
    page.getByRole('dialog', { name: 'settings-after.png' }),
  ).toHaveCount(0)

  await page.screenshot({ path: 'e2e/output/thread-media.png' })
})
