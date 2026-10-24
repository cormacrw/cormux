import { expect, test, type Locator, type Page } from '@playwright/test'
import { mkdirSync } from 'node:fs'

type Call = { cmd: string; args: Record<string, unknown> }

const iso = (date: Date) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`

/** This week's days, Monday first, as the page keys its lanes. */
function thisWeek() {
  const today = new Date()
  const monday = new Date(today)
  monday.setDate(today.getDate() - ((today.getDay() + 6) % 7))
  return Array.from({ length: 7 }, (_, index) => {
    const date = new Date(monday)
    date.setDate(monday.getDate() + index)
    return iso(date)
  })
}

async function openWeek(page: Page) {
  await page.getByRole('button', { name: /Search or run a command/ }).click()
  const field = page.getByRole('dialog').locator('input')
  await expect(field).toBeFocused()
  await page.keyboard.type('Go to week')
  await page.keyboard.press('Enter')
  await expect(
    page.getByRole('heading', { name: 'This week', level: 1 }),
  ).toBeVisible()
}

/** Pointer drag with real moves, so the 4px threshold and lane hit testing run. */
async function dragTo(page: Page, card: Locator, target: Locator) {
  const from = (await card.boundingBox())!
  await target.scrollIntoViewIfNeeded()
  await page.mouse.move(from.x + 40, from.y + from.height / 2)
  await page.mouse.down()
  await page.mouse.move(from.x + 46, from.y + from.height / 2 + 4)
  const to = (await target.boundingBox())!
  await page.mouse.move(to.x + to.width / 2, to.y + 40, { steps: 8 })
  await expect(target).toHaveAttribute('data-week-lane', /.+/)
  await page.mouse.up()
}

function calls(page: Page) {
  return page.evaluate(
    () =>
      (window as { __HARNESS_WEEK_CALLS__?: Call[] }).__HARNESS_WEEK_CALLS__ ??
      [],
  )
}

test.describe('week', () => {
  test('adds, plans, completes and edits a TODO', async ({ page }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    mkdirSync('e2e/output', { recursive: true })
    await page.goto('/')
    await openWeek(page)

    const days = thisWeek()
    const lane = (day: string) => page.locator(`[data-week-lane="${day}"]`)
    const loading = page.locator('[data-week-lane="loading"]')
    const progress = page.locator('[data-od-id="week-progress"]')

    // The fixture plans one TODO for Monday; today's lane is marked.
    await expect(lane(days[0]!)).toContainText('Draft the 0.4 changelog')
    await expect(page.locator('[data-today]')).toContainText('Today')
    await expect(progress).toHaveText('0 of 1 planned done')
    await expect(lane(days[6]!)).toContainText('Drop tasks here')

    // Enter adds a TODO to the loading area and keeps focus; blank input is ignored.
    const add = page.getByRole('textbox', { name: 'Add a TODO' })
    await add.fill('Book the venue')
    await add.press('Enter')
    await expect(add).toHaveValue('')
    await expect(add).toBeFocused()
    await expect(loading.locator('[data-week-item]')).toHaveCount(2)
    await expect(loading).toContainText('Book the venue')
    await add.press('Enter')
    await expect(loading.locator('[data-week-item]')).toHaveCount(2)
    // The newest TODO leads the loading area.
    const cardId = (await loading
      .locator('[data-week-item]')
      .first()
      .getAttribute('data-week-item'))!
    const newCard = page.locator(`[data-week-item="${cardId}"]`)

    // Drag it onto Tuesday.
    await dragTo(page, newCard, lane(days[1]!))
    await expect(lane(days[1]!)).toContainText('Book the venue')
    await expect(loading).not.toContainText('Book the venue')
    await expect(progress).toHaveText('0 of 2 planned done')

    // The check button completes it without opening the pane.
    const check = page.getByRole('button', {
      name: 'Mark “Book the venue” done',
    })
    await check.click()
    await expect(
      page.getByRole('button', { name: 'Mark “Book the venue” not done' }),
    ).toHaveAttribute('aria-pressed', 'true')
    await expect(newCard).toHaveAttribute('data-done', 'true')
    await expect(page.locator('[data-od-id="week-pane"]')).toHaveCount(0)
    await expect(progress).toHaveText('1 of 2 planned done')

    // Clicking the card opens the pane, where the description saves after typing stops.
    await newCard.locator('[data-week-card]').click()
    const pane = page.locator('[data-od-id="week-pane"]')
    await expect(pane.getByRole('textbox', { name: 'Title' })).toHaveValue(
      'Book the venue',
    )
    await expect(
      pane.getByRole('button', { name: 'Completed · mark not done' }),
    ).toBeVisible()
    await pane
      .getByRole('textbox', { name: 'Description' })
      .fill('Two quotes, then call on Thursday.')
    await expect
      .poll(async () =>
        (await calls(page)).some(
          (call) =>
            call.cmd === 'update_todo' &&
            call.args.description === 'Two quotes, then call on Thursday.',
        ),
      )
      .toBe(true)

    // "Planned for" moves it to another day.
    const radios = pane.getByRole('radiogroup', { name: 'Planned for' })
    await expect(radios.getByRole('radio', { checked: true })).toHaveAttribute(
      'data-plan-day',
      days[1]!,
    )
    await radios.locator(`[data-plan-day="${days[3]}"]`).click()
    await expect(lane(days[3]!)).toContainText('Book the venue')
    await expect(lane(days[1]!)).not.toContainText('Book the venue')
    await page.screenshot({ path: 'e2e/output/week.png' })

    // Escape closes the pane and puts focus back on the card.
    await page.keyboard.press('Escape')
    await expect(pane).toHaveCount(0)
    await expect(newCard.locator('[data-week-card]')).toBeFocused()

    // Reopening shows the saved description.
    await newCard.locator('[data-week-card]').click()
    await expect(
      pane.getByRole('textbox', { name: 'Description' }),
    ).toHaveValue('Two quotes, then call on Thursday.')
    await pane.getByRole('radio', { name: 'Unplanned' }).click()
    await expect(loading).toContainText('Book the venue')

    expect(errors).toEqual([])
  })

  test('plans sprint tasks locally, without writing to ClickUp', async ({
    page,
  }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    await page.addInitScript(() => {
      ;(window as { __HARNESS_CLICKUP__?: boolean }).__HARNESS_CLICKUP__ = true
    })
    await page.goto('/')
    await openWeek(page)

    const days = thisWeek()
    const today = iso(new Date())
    const loading = page.locator('[data-week-lane="loading"]')
    const sprintCard = page.locator('[data-week-item="sprint:t2"]')
    await expect(sprintCard).toContainText('ENG-198')
    await expect(loading).toContainText('Session expires mid checkout')
    // The harness plans t3 for today and finished t6 (done in ClickUp) for Monday.
    await expect(
      page.locator(`[data-week-lane="${today}"] [data-week-item="sprint:t3"]`),
    ).toBeVisible()
    await expect(
      page.locator(
        `[data-week-lane="${days[0]}"] [data-week-item="sprint:t6"]`,
      ),
    ).toHaveAttribute('data-done', 'true')

    // The filter narrows the loading area.
    await loading.getByRole('button', { name: 'TODOs', exact: true }).click()
    await expect(sprintCard).toHaveCount(0)
    await loading.getByRole('button', { name: 'Sprint', exact: true }).click()
    await expect(sprintCard).toBeVisible()
    await expect(loading.locator('[data-week-item^="todo:"]')).toHaveCount(0)
    await loading.getByRole('button', { name: 'All', exact: true }).click()

    // The pane is the Sprint page's, plus the plan controls.
    await sprintCard.locator('[data-week-card]').click()
    const pane = page.locator('[data-od-id="week-pane"]')
    await expect(
      pane.getByRole('heading', {
        name: 'Session expires mid checkout on Safari',
      }),
    ).toBeVisible()
    await pane.locator(`[data-plan-day="${days[4]}"]`).click()
    await expect(
      page.locator(
        `[data-week-lane="${days[4]}"] [data-week-item="sprint:t2"]`,
      ),
    ).toBeVisible()
    await pane.getByRole('button', { name: 'Mark complete' }).click()
    await expect(sprintCard).toHaveAttribute('data-done', 'true')

    const clickupWrites = await page.evaluate(
      () =>
        (window as { __HARNESS_CLICKUP_CALLS__?: Call[] })
          .__HARNESS_CLICKUP_CALLS__ ?? [],
    )
    expect(clickupWrites).toEqual([])
    expect((await calls(page)).map((call) => call.cmd)).toEqual([
      'plan_sprint_task',
      'set_sprint_task_done',
    ])
    await page.screenshot({ path: 'e2e/output/week-sprint.png' })

    expect(errors).toEqual([])
  })
})
