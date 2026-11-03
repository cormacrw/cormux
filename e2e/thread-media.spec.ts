import { expect, test } from '@playwright/test'
import { setScenario } from './harness'

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
  await setScenario(page, {
    timeline: [
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
  })
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
  const viewer = page.getByRole('dialog', { name: 'settings-after.png' })
  await expect(viewer).toBeVisible()

  // The viewer covers the workspace column (header included) but stops at
  // the sidebar, and sits above the workspace header.
  const host = page.locator('[data-media-lightbox-host]')
  const viewerBox = await viewer.boundingBox()
  const hostBox = await host.boundingBox()
  expect(viewerBox).toEqual(hostBox)
  expect(viewerBox!.x).toBeGreaterThan(0)
  const covering = await page.evaluate(
    ({ x, y }) =>
      document
        .elementFromPoint(x, y)
        ?.closest('[role="dialog"]')
        ?.getAttribute('aria-label') ?? null,
    { x: hostBox!.x + hostBox!.width / 2, y: hostBox!.y + 10 },
  )
  expect(covering).toBe('settings-after.png')
  await page.screenshot({ path: 'e2e/output/thread-media-viewer.png' })

  await viewer.getByRole('button', { name: 'Zoom in' }).click()
  await viewer.getByRole('button', { name: 'Zoom in' }).click()
  await expect(viewer.getByRole('button', { name: 'Reset zoom' })).toHaveText(
    '225%',
  )
  await page.waitForTimeout(150)
  await page.screenshot({ path: 'e2e/output/thread-media-zoomed.png' })
  await page.keyboard.press('0')
  await expect(viewer.getByRole('button', { name: 'Reset zoom' })).toHaveText(
    '100%',
  )

  await page.getByRole('button', { name: 'Close' }).click()
  await expect(
    page.getByRole('dialog', { name: 'settings-after.png' }),
  ).toHaveCount(0)

  await page.screenshot({ path: 'e2e/output/thread-media.png' })
})
