import { expect, test, type Page } from '@playwright/test'
import { mkdirSync } from 'node:fs'

type Call = { cmd: string; args: Record<string, unknown> }

async function enableClickup(page: Page) {
  await page.addInitScript(() => {
    ;(window as { __HARNESS_CLICKUP__?: boolean }).__HARNESS_CLICKUP__ = true
  })
}

function calls(page: Page) {
  return page.evaluate(
    () =>
      (window as { __HARNESS_CLICKUP_CALLS__?: Call[] })
        .__HARNESS_CLICKUP_CALLS__ ?? [],
  )
}

test.describe('clickup', () => {
  test('stays hidden without an API key', async ({ page }) => {
    await page.goto('/')
    const nav = page.getByRole('navigation', { name: 'Harness' })
    await expect(nav.getByRole('button', { name: /TODOs/ })).toBeVisible()
    await expect(nav.getByRole('button', { name: /Sprint/ })).toHaveCount(0)
  })

  test('saving a key in Settings reveals the Sprint page', async ({ page }) => {
    await page.goto('/')
    const nav = page.getByRole('navigation', { name: 'Harness' })
    await nav.getByRole('button', { name: /Settings/ }).click()
    await page
      .getByRole('navigation', { name: 'Settings sections' })
      .getByRole('button', { name: 'ClickUp' })
      .click()
    const key = page.locator('#settings-clickup-key')

    await key.fill('not-a-key')
    await page.getByRole('button', { name: 'Save' }).click()
    await expect(page.getByRole('alert')).toHaveText(
      'ClickUp rejected the API key',
    )

    await key.fill('pk_123')
    await page.getByRole('button', { name: 'Save' }).click()
    await expect(page.getByText('Connected', { exact: true })).toBeVisible()
    await expect(nav.getByRole('button', { name: /Sprint/ })).toBeVisible()
    // One workspace and space pick themselves; the folder named Sprints wins over Roadmap.
    await expect(
      page.locator('[data-od-id="settings-clickup-folder"]'),
    ).toHaveText('Sprints')
  })

  test('board, drag between lanes and points', async ({ page, context }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write'])
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    mkdirSync('e2e/output', { recursive: true })
    await enableClickup(page)
    await page.goto('/')

    // Opening a card shows its details in the pane.
    await page
      .getByRole('navigation', { name: 'Harness' })
      .getByRole('button', { name: /Sprint/ })
      .click()
    await expect(
      page.getByRole('heading', { name: 'Sprint 14', level: 1 }),
    ).toBeVisible()
    await page.locator('[data-task-id="t2"]').click()
    const panel = page.locator('[data-od-id="sprint-task-panel"]')
    await expect(
      panel.getByRole('heading', {
        name: 'Session expires mid checkout on Safari',
      }),
    ).toBeVisible()
    await expect(panel.getByText('Customers lose their cart')).toBeVisible()

    // The description renders as rich markdown.
    await expect(panel.getByRole('heading', { name: 'Problem' })).toBeVisible()
    await expect(panel.getByRole('cell', { name: 'Safari 18' })).toBeVisible()
    await expect(panel.getByRole('checkbox')).toHaveCount(2)
    await expect(panel.locator('strong')).toHaveText('mid checkout')

    // The ID copies in one click.
    await panel.getByRole('button', { name: 'Copy ENG-198' }).click()
    await expect(
      panel.getByRole('button', { name: 'Copied ENG-198' }),
    ).toBeVisible()
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(
      'ENG-198',
    )

    // Without a custom ID the card shows ClickUp's own.
    await expect(page.locator('[data-task-id="t4"]')).toHaveAccessibleName(
      /^t4 Audit log/,
    )
    await expect(
      page.getByRole('button', { name: 'Open t4 in ClickUp' }),
    ).toBeAttached()

    // Unpointed tasks stand out on the board and in the header.
    const unpointed = page.locator('[data-unpointed]')
    await expect(unpointed).toHaveCount(2)
    await expect(
      page.locator('[data-od-id="sprint-unpointed-count"]'),
    ).toHaveText(/2 tasks without points/)
    await page.screenshot({ path: 'e2e/output/clickup-board.png' })

    // Estimating from the pane clears the highlight.
    await panel.getByRole('button', { name: '3 points', exact: true }).click()
    await expect(unpointed).toHaveCount(1)
    await expect(
      panel.getByRole('button', { name: '3 points', exact: true }),
    ).toHaveAttribute('aria-pressed', 'true')

    // Drag ENG-201 from To do into In progress.
    const card = page.locator('[data-task-id="t1"]')
    const target = page.locator('[data-lane="in progress"]')
    const from = (await card.boundingBox())!
    const to = (await target.boundingBox())!
    await page.mouse.move(from.x + 20, from.y + 20)
    await page.mouse.down()
    await page.mouse.move(from.x + 40, from.y + 30, { steps: 4 })
    await page.mouse.move(to.x + to.width / 2, to.y + 120, { steps: 10 })
    await page.mouse.up()
    await expect(target.locator('[data-task-id="t1"]')).toBeVisible()

    // The sidebar's in-progress count picks up the moved task straight away.
    await expect(
      page
        .getByRole('navigation', { name: 'Harness' })
        .getByRole('button', { name: /Sprint/ }),
    ).toContainText('4')

    expect(await calls(page)).toEqual([
      {
        cmd: 'set_clickup_task_points',
        args: { taskId: 't2', points: 3 },
      },
      {
        cmd: 'set_clickup_task_status',
        args: { taskId: 't1', status: 'in progress' },
      },
    ])

    // A drag doesn't open the dragged card; Escape closes the pane.
    await expect(
      panel.getByRole('heading', {
        name: 'Session expires mid checkout on Safari',
      }),
    ).toBeVisible()
    await page.keyboard.press('Escape')
    await expect(panel).toHaveCount(0)
    await expect(page.locator('[data-task-id="t2"]')).toBeFocused()

    expect(errors).toEqual([])
  })

  test('hides and shows lanes', async ({ page }) => {
    await enableClickup(page)
    await page.goto('/')
    await page
      .getByRole('navigation', { name: 'Harness' })
      .getByRole('button', { name: /Sprint/ })
      .click()
    const lanes = page.locator('[data-lane]')
    await expect(lanes).toHaveCount(4)

    // The lane's own button hides it, and the menu counts it.
    const complete = page.locator('[data-lane="complete"]')
    await complete.hover()
    await complete.getByRole('button', { name: 'Hide Complete' }).click()
    await expect(complete).toHaveCount(0)
    const menu = page.locator('[data-od-id="sprint-lanes-menu"]')
    await expect(menu).toContainText('1 hidden')

    // The menu toggles lanes and stays open between toggles.
    await menu.click()
    await page.getByRole('menuitemcheckbox', { name: /In review/ }).click()
    await expect(page.locator('[data-lane="in review"]')).toHaveCount(0)
    await expect(menu).toContainText('2 hidden')
    await page.getByRole('menuitemcheckbox', { name: /Complete/ }).click()
    await expect(complete).toBeVisible()
    await page.getByRole('menuitem', { name: 'Show all lanes' }).click()
    await expect(lanes).toHaveCount(4)
    await expect(menu).not.toContainText('hidden')
  })
})
