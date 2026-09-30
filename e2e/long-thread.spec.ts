import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

const gapFromBottom = (page: import('@playwright/test').Page) =>
  page.evaluate(() => {
    const scroll = document.getElementById('thread-scroll')!
    return scroll.scrollHeight - scroll.scrollTop - scroll.clientHeight
  })

test('a long thread renders its newest turn and follows a streamed reply', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/?longThread=200')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  const timeline = page.locator('#timeline')
  await expect(
    timeline.getByText('Answer 200', { exact: true }),
  ).toBeInViewport()

  const composer = page.getByRole('textbox', { name: 'Message the agent' })
  await composer.fill('a brand new message')
  await page.getByRole('button', { name: 'Send to agent' }).click()
  await expect(timeline.getByText('a brand new message')).toBeInViewport()
  // Replies type out after they stream in, so the last words land a few seconds later.
  await expect(timeline.getByText(/done\.$/)).toBeInViewport({
    timeout: 20_000,
  })
  expect(await gapFromBottom(page)).toBeLessThan(2)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/long-thread-sent.png' })

  // Scrolled up to read, a streaming reply must not pull the view back down.
  await composer.fill('second message')
  await page.getByRole('button', { name: 'Send to agent' }).click()
  // A real wheel: scrollTop set from code is how the virtual list moves too, so it doesn't unpin.
  await page.locator('#thread-scroll').hover()
  await page.mouse.wheel(0, -4000)
  await expect(page.getByText('1 working')).toBeVisible()
  await expect(page.getByText('0 working')).toBeVisible({ timeout: 5000 })
  expect(await gapFromBottom(page)).toBeGreaterThan(1000)
  expect(errors).toEqual([])
})

test('small trackpad steps scroll up out of a streaming reply', async ({
  page,
}) => {
  await page.goto('/?longThread=200')
  await page.getByRole('button', { name: 'OAuth login, Idle, 1 agent' }).click()
  const timeline = page.locator('#timeline')
  await expect(
    timeline.getByText('Answer 200', { exact: true }),
  ).toBeInViewport()

  await page
    .getByRole('textbox', { name: 'Message the agent' })
    .fill('stream please')
  await page.getByRole('button', { name: 'Send to agent' }).click()
  await expect(page.getByText('1 working')).toBeVisible()
  // Each step stays well inside the 80px re-pin zone; following used to snap every one back.
  await page.locator('#thread-scroll').hover()
  for (let step = 0; step < 20; step += 1) {
    await page.mouse.wheel(0, -15)
    await page.waitForTimeout(30)
  }
  await expect(page.getByText('0 working')).toBeVisible({ timeout: 5000 })
  expect(await gapFromBottom(page)).toBeGreaterThan(150)
})
