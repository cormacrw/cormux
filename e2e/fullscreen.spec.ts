import { expect, test } from '@playwright/test'
import { mkdirSync, readFileSync } from 'node:fs'

test('the sidebar keeps room for the traffic lights, with DEV in its footer', async ({
  page,
}) => {
  await page.goto('/')
  // Flush to the window edges, so the native traffic lights (inset in
  // tauri.conf.json) shift by the 12px float margin they used to sit inside.
  const sidebarBox = await page.locator('.app-shell > aside').boundingBox()
  expect(sidebarBox?.x).toBe(0)
  expect(sidebarBox?.y).toBe(0)
  expect(sidebarBox!.y + sidebarBox!.height).toBe(
    await page.evaluate(() => window.innerHeight),
  )
  const tauriConf = JSON.parse(
    readFileSync('src-tauri/tauri.conf.json', 'utf8'),
  ) as {
    app: { windows: { trafficLightPosition: { x: number; y: number } }[] }
  }
  expect(tauriConf.app.windows[0].trafficLightPosition).toEqual({
    x: 22,
    y: 28,
  })
  const titlebar = page.locator('[data-od-id="sidebar-titlebar"]')
  await expect(titlebar).toBeVisible()
  expect((await titlebar.boundingBox())?.height).toBe(52)
  const commandBar = page.locator('[data-od-id="sidebar-command-bar"]')
  const settings = page.getByRole('button', { name: 'Settings' })
  const badge = page.locator('[data-od-id="dev-build-badge"]')
  await expect(badge).toBeVisible()
  // DEV sits in the footer row, beside Settings rather than under the traffic lights.
  const badgeBox = await badge.boundingBox()
  const settingsBox = await settings.boundingBox()
  expect(badgeBox!.y).toBeGreaterThan(settingsBox!.y - 4)
  expect((await commandBar.boundingBox())!.y).toBeGreaterThanOrEqual(52)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/windowed-sidebar.png' })
})

test('full screen drops the traffic-light row so the sidebar moves up', async ({
  page,
}) => {
  await page.addInitScript(() => {
    ;(window as { __HARNESS_FULLSCREEN__?: boolean }).__HARNESS_FULLSCREEN__ =
      true
  })
  await page.goto('/')
  const titlebar = page.locator('[data-od-id="sidebar-titlebar"]')
  await expect.poll(async () => (await titlebar.boundingBox())?.height).toBe(14)
  const commandBar = page.locator('[data-od-id="sidebar-command-bar"]')
  expect((await commandBar.boundingBox())!.y).toBeLessThan(30)
  await expect(page.locator('[data-od-id="dev-build-badge"]')).toBeVisible()
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/fullscreen-sidebar.png' })
})
