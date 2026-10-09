import { expect, test } from '@playwright/test'

test('Dragging the sidebar edge resizes it, and the width persists', async ({
  page,
}) => {
  await page.goto('/')
  const sidebar = page.locator('.app-shell > aside')
  const handle = page.getByRole('separator', { name: 'Resize sidebar' })
  const before = (await sidebar.boundingBox())!.width

  const box = (await handle.boundingBox())!
  await page.mouse.move(box.x + box.width / 2, box.y + 200)
  await page.mouse.down()
  await page.mouse.move(box.x + 100, box.y + 200, { steps: 5 })
  await page.mouse.up()
  await expect
    .poll(async () => (await sidebar.boundingBox())!.width)
    .toBeGreaterThan(before + 80)

  const resized = (await sidebar.boundingBox())!.width
  await page.reload()
  await expect
    .poll(async () => (await sidebar.boundingBox())!.width)
    .toBeCloseTo(resized, 0)

  await handle.dblclick()
  await expect
    .poll(async () => (await sidebar.boundingBox())!.width)
    .toBeCloseTo(before, 0)
})
