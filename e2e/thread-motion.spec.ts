import { expect, test, type Page } from '@playwright/test'

/**
 * Records the text of every timeline row that starts a Web Animation (a pop-in), as it starts.
 * A pop lasts a third of a second, so polling for running ones can miss it.
 */
const recordPops = (page: Page) =>
  page.addInitScript(() => {
    const w = window as Window & { __pops?: string[] }
    w.__pops = []
    const animate = Element.prototype.animate
    Element.prototype.animate = function (...args) {
      if (this.closest('#timeline')) {
        w.__pops!.push((this.textContent ?? '').trim())
      }
      return animate.apply(this, args)
    }
  })

const pops = (page: Page) =>
  page.evaluate(() => (window as Window & { __pops?: string[] }).__pops ?? [])

const replyLength = (page: Page) =>
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
  await recordPops(page)
  await page.goto('/?longThread=3')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(
    page.locator('#timeline').getByText('Answer 3', { exact: true }),
  ).toBeVisible()
  // Rows already in the thread when it opens stay still.
  expect(await pops(page)).toEqual([])

  // The reply's length on every frame from here, to see it reveal.
  await page.evaluate(() => {
    const w = window as Window & { __lengths?: number[] }
    w.__lengths = []
    const sample = () => {
      const reply = [
        ...document.querySelectorAll('#timeline .agent-markdown'),
      ].at(-1)
      w.__lengths!.push(reply?.textContent?.length ?? 0)
      requestAnimationFrame(sample)
    }
    requestAnimationFrame(sample)
  })
  await page.locator('#composer-input').fill('Go')
  await page.keyboard.press('Enter')
  await expect
    .poll(() => pops(page))
    .toEqual(
      expect.arrayContaining([
        expect.stringContaining('Go'),
        expect.stringContaining('src/lib/auth.ts'),
      ]),
    )
  await expect(page.locator('#timeline').getByText(/done\.$/)).toBeVisible({
    timeout: 20_000,
  })
  await expect(page.locator('[data-od-id="composer-stop"]')).toHaveCount(0)

  // The reply was revealed a little at a time, then in full.
  const full = await replyLength(page)
  const lengths = await page.evaluate(
    () => (window as Window & { __lengths?: number[] }).__lengths ?? [],
  )
  expect(lengths.some((length) => length > 0 && length < full)).toBe(true)

  // Reloading the turn from the store once it ends replays nothing: the prompt popped once.
  expect((await pops(page)).filter((text) => text.includes('Go'))).toHaveLength(
    1,
  )
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
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
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
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(page.locator('#composer-input')).toBeVisible()
  const long = Array.from(
    { length: 30 },
    (_, n) =>
      `Paragraph ${n + 1} explains one more part of the change in plain words.`,
  ).join('\n\n')
  const started = Date.now()
  await page.evaluate(
    (events) => window.__HARNESS__!.agentEvents(events),
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

test('a reply that lands in a thread the agent isn’t writing in shows in full', async ({
  page,
}) => {
  await page.goto('/?longThread=3')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(
    page.locator('#timeline').getByText('Answer 3', { exact: true }),
  ).toBeVisible()
  // What a snapshot does on opening a thread that moved on while it was hidden.
  const late = `Finished while you were away. ${'more words here '.repeat(60)}`
  // Sample frames from the moment it lands: it never shows the typing cursor.
  const typedFrames = await page.evaluate(
    async (events) => {
      window.__HARNESS__!.agentEvents(events)
      let typed = 0
      for (let frame = 0; frame < 20; frame += 1) {
        await new Promise((done) => requestAnimationFrame(done))
        if (document.querySelector('#timeline .type-cursor')) typed += 1
      }
      return typed
    },
    [agentSays(late)],
  )
  expect(typedFrames).toBe(0)
  expect(await replyLength(page)).toBeGreaterThanOrEqual(late.trim().length)
})

test('two replies never type at the same time', async ({ page }) => {
  await page.goto('/')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  await expect(page.locator('#composer-input')).toBeVisible()
  const say = (events: object[]) =>
    page.evaluate((events) => window.__HARNESS__!.agentEvents(events), events)
  // A reply, a skill's step, then the skill's own output straight after.
  await say([agentSays(`First reply. ${'more words here '.repeat(60)}`)])
  await say([
    {
      type: 'toolCall',
      id: 'skill-1',
      title: 'Skill',
      role: { kind: 'skill', name: 'release-notes' },
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
  const replyText = (start: string) =>
    page
      .locator('#timeline .agent-markdown', { hasText: start })
      .evaluate((el) => (el.textContent ?? '').trim())
  await expect
    .poll(() => replyText('First reply.'))
    .toBe(`First reply. ${'more words here '.repeat(60)}`.trim())
  await expect
    .poll(() => replyText('Second reply.'), { timeout: 3000 })
    .toBe(`Second reply. ${'other words here '.repeat(60)}`.trim())
})
