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
