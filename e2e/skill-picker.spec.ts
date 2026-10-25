import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('typing / anywhere in a Claude composer offers skills and inserts the pick', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  mkdirSync('e2e/output', { recursive: true })
  await page.goto('/')

  await page
    .getByRole('button', {
      name: 'Why the webhook signature fails, my-app, Idle',
    })
    .click()
  const input = page.locator('#composer-input')
  await input.fill('/')
  const picker = page.getByRole('listbox', { name: 'Skills' })
  await expect(picker.getByRole('option')).toHaveCount(3)
  await page.waitForTimeout(200)
  await page.screenshot({ path: 'e2e/output/skill-picker.png' })

  await input.pressSequentially('rev')
  await expect(picker.getByRole('option')).toHaveCount(1)
  await input.press('Enter')
  await expect(input).toHaveValue('/code-review ')
  await expect(picker).toHaveCount(0)

  // A slash after other text opens it too, and the pick replaces just that word.
  await input.fill('please run ')
  await input.pressSequentially('/rev')
  await expect(picker.getByRole('option')).toHaveCount(1)
  await input.press('Enter')
  await expect(input).toHaveValue('please run /code-review ')
  await input.pressSequentially('then src/app')
  await expect(picker).toHaveCount(0)

  // Esc closes the picker without stopping anything.
  await input.fill('/')
  await expect(picker).toBeVisible()
  await input.press('Escape')
  await expect(picker).toHaveCount(0)

  expect(errors).toEqual([])
})

test('@ offers the repos set up in Cormux and sends the alias as the repo’s folder', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login', exact: true }).click()
  const input = page.locator('#composer-input')
  await input.click()
  await input.pressSequentially('Copy the toast from @my')
  const picker = page.getByRole('listbox', { name: 'Repositories' })
  await expect(picker.getByRole('option')).toHaveText([/@my-app/])
  await input.press('Enter')
  await expect(input).toHaveValue('Copy the toast from @my-app ')

  await input.pressSequentially('and mail me@my-app.dev')
  await expect(picker).toHaveCount(0)
  await input.press('Enter')
  const prompts = await page.evaluate(
    () =>
      (window as { __HARNESS_PROMPTS__?: { text: string }[] })
        .__HARNESS_PROMPTS__,
  )
  // The fixture repo's folder; the email stays as typed.
  expect(prompts!.at(-1)!.text).toBe(
    'Copy the toast from /tmp/cormux-fixture/my-app and mail me@my-app.dev',
  )
})

test('# offers the sprint’s tasks and sends one as its name, link and description', async ({
  page,
}) => {
  await page.addInitScript(() => {
    ;(window as { __HARNESS_CLICKUP__?: boolean }).__HARNESS_CLICKUP__ = true
  })
  await page.goto('/')
  await page.getByRole('button', { name: 'OAuth login', exact: true }).click()
  const input = page.locator('#composer-input')
  await input.click()
  await input.pressSequentially('Fix #eng-19')
  const picker = page.getByRole('listbox', { name: 'Sprint tasks' })
  await expect(picker.getByRole('option')).toHaveText([
    /#ENG-198\s*Session expires mid checkout on Safari\s*in progress/,
    /#ENG-190\s*Move invoice PDFs/,
  ])
  await input.press('Enter')
  await expect(input).toHaveValue('Fix #ENG-198 ')

  // A heading isn't a task.
  await input.pressSequentially('and keep # notes')
  await expect(picker).toHaveCount(0)
  await input.press('Enter')
  await expect
    .poll(async () => {
      const prompts = await page.evaluate(
        () =>
          (window as { __HARNESS_PROMPTS__?: { text: string }[] })
            .__HARNESS_PROMPTS__,
      )
      return prompts?.at(-1)?.text ?? ''
    })
    .toMatch(
      /^Fix ENG-198 “Session expires mid checkout on Safari” \(https:\/\/[^)]+\) and keep # notes\n\nClickUp tasks:\n\n### ENG-198 Session expires mid checkout on Safari\n## Problem/,
    )
})

test('the picker opening under a still pointer keeps the first option highlighted', async ({
  page,
}) => {
  await page.goto('/')
  await page
    .getByRole('button', {
      name: 'Why the webhook signature fails, my-app, Idle',
    })
    .click()
  const input = page.locator('#composer-input')
  // Park the pointer just above the prompt bar, where the last option will open.
  const bar = (await page.locator('#composer').boundingBox())!
  await page.mouse.move(bar.x + bar.width / 2, bar.y - 24)
  await input.fill('/')
  const options = page
    .getByRole('listbox', { name: 'Skills' })
    .getByRole('option')
  await expect(options).toHaveCount(3)
  await page.waitForTimeout(300)
  await expect(options.first()).toHaveAttribute('aria-selected', 'true')
  await expect(options.last()).toHaveAttribute('aria-selected', 'false')

  // Moving the pointer does pick what's under it.
  const last = (await options.last().boundingBox())!
  await page.mouse.move(last.x + 40, last.y + last.height / 2, { steps: 3 })
  await expect(options.last()).toHaveAttribute('aria-selected', 'true')
})
