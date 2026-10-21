import { expect, test } from '@playwright/test'

// Text of the elements running a pop-in (Web Animations, not CSS transitions or keyframes).
const popping = (page: import('@playwright/test').Page) =>
  page.evaluate(() =>
    document
      .getAnimations()
      .filter((animation) => animation.constructor.name === 'Animation')
      .map((animation) =>
        ((animation.effect as KeyframeEffect).target?.textContent ?? '').trim(),
      ),
  )

const replyLength = (page: import('@playwright/test').Page) =>
  page.evaluate(
    () =>
      (
        [...document.querySelectorAll('#timeline .agent-markdown')].at(-1)
          ?.textContent ?? ''
      ).length,
  )

test('only new rows pop in, and a new reply types out', async ({ page }) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/?longThread=3')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await expect(
    page.locator('#timeline').getByText('Answer 3', { exact: true }),
  ).toBeVisible()
  // Rows already in the thread when it opens stay still.
  expect(await popping(page)).toEqual([])

  await page.locator('#composer-input').fill('Go')
  await page.keyboard.press('Enter')
  await expect
    .poll(() => popping(page))
    .toEqual(expect.arrayContaining([expect.stringContaining('Go')]))
  await expect
    .poll(() => popping(page))
    .toEqual(
      expect.arrayContaining([expect.stringContaining('src/lib/auth.ts')]),
    )

  // The reply is revealed a little at a time, then in full.
  await expect.poll(() => replyLength(page)).toBeGreaterThan(0)
  const early = await replyLength(page)
  await expect.poll(() => replyLength(page)).toBeGreaterThan(early)
  await expect(page.locator('#timeline').getByText(/done\.$/)).toBeVisible({
    timeout: 20_000,
  })

  // Reloading the turn from the store once it ends replays nothing.
  await expect(page.getByText('0 working')).toBeVisible({ timeout: 5000 })
  expect(await popping(page)).toEqual([])
  expect(errors).toEqual([])
})

test('a thread with thousands of events keeps drawing frames while a reply streams', async ({
  page,
}) => {
  await page.addInitScript(() => {
    const w = window as Window & { __frames?: number }
    w.__frames = 0
    const tick = () => {
      w.__frames! += 1
      requestAnimationFrame(tick)
    }
    requestAnimationFrame(tick)
  })
  await page.goto('/?longThread=3000')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  await page.locator('#composer-input').fill('Go')
  const frames = () =>
    page.evaluate(
      () => (window as Window & { __frames?: number }).__frames ?? 0,
    )
  const before = await frames()
  await page.keyboard.press('Enter')
  await page.waitForTimeout(500)
  // Rebuilding the timeline on every chunk must stay cheap, or the typing and follow-scroll stall.
  expect((await frames()) - before).toBeGreaterThan(15)
})

const agentSays = (text: string) => ({
  type: 'messageChunk',
  role: 'agent',
  text,
})

test('a long reply the agent sent all at once doesn’t take ages to type out', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login', exact: true }).click()
  await expect(page.locator('#composer-input')).toBeVisible()
  const long = Array.from(
    { length: 30 },
    (_, n) =>
      `Paragraph ${n + 1} explains one more part of the change in plain words.`,
  ).join('\n\n')
  const started = Date.now()
  await page.evaluate(
    (events) =>
      (window as { __HARNESS_AGENT_EVENTS__?: (events: object[]) => void })
        .__HARNESS_AGENT_EVENTS__!(events),
    [agentSays(long)],
  )
  // About 2,200 characters: the old 100-a-second cap took over 20 seconds.
  await expect(
    page.locator('#timeline').getByText('Paragraph 30 explains'),
  ).toBeVisible({
    timeout: 3000,
  })
  expect(Date.now() - started).toBeLessThan(3000)
})

test('two replies never type at the same time', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login', exact: true }).click()
  await expect(page.locator('#composer-input')).toBeVisible()
  const say = (events: object[]) =>
    page.evaluate(
      (events) =>
        (window as { __HARNESS_AGENT_EVENTS__?: (events: object[]) => void })
          .__HARNESS_AGENT_EVENTS__!(events),
      events,
    )
  // A reply, a skill's step, then the skill's own output straight after.
  await say([agentSays(`First reply. ${'more words here '.repeat(60)}`)])
  await say([
    {
      type: 'toolCall',
      id: 'skill-1',
      title: 'Skill',
      name: 'Skill',
      kind: 'other',
      status: 'completed',
      locations: [],
      detail: 'release-notes',
    },
    agentSays(`Second reply. ${'other words here '.repeat(60)}`),
  ])
  // Sample frames while they type: at most one reply has the typing cursor at once.
  const most = await page.evaluate(async () => {
    let peak = 0
    for (let frame = 0; frame < 90; frame += 1) {
      await new Promise((done) => requestAnimationFrame(done))
      peak = Math.max(
        peak,
        document.querySelectorAll('#timeline .type-cursor').length,
      )
    }
    return peak
  })
  expect(most).toBeLessThanOrEqual(1)
  // Both end up in full.
  const replyText = (key: string) =>
    page
      .locator(`[data-row-key="${key}"] .agent-markdown`)
      .evaluate((el) => (el.textContent ?? '').trim())
  await expect
    .poll(() => replyText('thought-1'))
    .toBe(`First reply. ${'more words here '.repeat(60)}`.trim())
  await expect
    .poll(() => replyText('thought-2'), { timeout: 3000 })
    .toBe(`Second reply. ${'other words here '.repeat(60)}`.trim())
})
