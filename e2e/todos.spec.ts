import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test.describe('todos', () => {
  test('adds, pins, unpins and deletes tasks', async ({ page }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    mkdirSync('e2e/output', { recursive: true })
    await page.goto('/')

    // The fixture's pinned task sits above Scratches on Homebase.
    const pinned = page.locator('[data-od-id="home-pinned-todos"]')
    await expect(
      pinned.getByText('Rotate the Stripe webhook secret'),
    ).toBeVisible()
    const scratches = page.locator('[data-od-id="home-sessions"]')
    expect((await pinned.boundingBox())!.y).toBeLessThan(
      (await scratches.boundingBox())!.y,
    )

    const nav = page.getByRole('navigation', { name: 'Harness' })
    await nav.getByRole('button', { name: /TODOs/ }).click()
    await expect(page.getByRole('heading', { name: 'TODOs' })).toBeVisible()
    const add = page.getByRole('textbox', { name: 'Add a task' })
    await expect(add).toBeFocused()

    await add.fill('Bump Tauri to 2.9')
    await add.press('Enter')
    await expect(add).toHaveValue('')
    await expect(add).toBeFocused()
    const rows = page.locator('[data-todo-id]')
    await expect(rows).toHaveCount(3)
    await expect(rows.last()).toContainText('Bump Tauri to 2.9')

    // Blank titles are ignored.
    await add.press('Enter')
    await expect(rows).toHaveCount(3)

    await page
      .getByRole('button', { name: 'Pin Bump Tauri to 2.9 to Homebase' })
      .click()
    await expect(
      page.getByRole('button', {
        name: 'Unpin Bump Tauri to 2.9 from Homebase',
      }),
    ).toHaveAttribute('aria-pressed', 'true')
    await page.screenshot({ path: 'e2e/output/todos-page.png', fullPage: true })

    await page
      .getByRole('button', { name: 'Delete Draft the 0.4 changelog' })
      .click()
    await expect(rows).toHaveCount(2)

    await nav.getByRole('button', { name: /Homebase/ }).click()
    await expect(pinned.locator('[data-pinned-todo]')).toHaveCount(2)
    await expect(pinned.getByText('Bump Tauri to 2.9')).toBeVisible()
    await page.screenshot({ path: 'e2e/output/todos-home.png', fullPage: true })

    // Unpinning every card removes the section.
    await pinned
      .getByRole('button', { name: 'Unpin Bump Tauri to 2.9' })
      .click()
    await pinned
      .getByRole('button', { name: 'Unpin Rotate the Stripe webhook secret' })
      .click()
    await expect(pinned).toHaveCount(0)

    expect(errors).toEqual([])
  })

  test('adds a task from the command palette', async ({ page }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    mkdirSync('e2e/output', { recursive: true })
    await page.goto('/')

    const chip = page.locator('[data-od-id="palette-todo-chip"]')
    const dialog = page.getByRole('dialog')
    const field = dialog.locator('input')
    // Focus moves into the dialog a beat after it opens.
    const openPalette = async () => {
      await page
        .getByRole('button', { name: /Search or run a command/ })
        .click()
      await expect(field).toBeFocused()
    }

    // "todo" + Space turns into a chip and clears the field.
    await openPalette()
    await page.keyboard.type('todo ')
    await expect(chip).toBeVisible()
    await expect(field).toHaveValue('')

    // Backspace on an empty field turns the chip back into text.
    await page.keyboard.press('Backspace')
    await expect(chip).toHaveCount(0)
    await expect(field).toHaveValue('todo')

    // Tab and Enter work too.
    await page.keyboard.press('Tab')
    await expect(chip).toBeVisible()
    await expect(field).toBeFocused()
    await page.keyboard.type('Renew the Apple cert')
    await expect(dialog.getByText('Add “Renew the Apple cert”')).toBeVisible()
    await page.screenshot({ path: 'e2e/output/todos-palette.png' })
    await page.keyboard.press('Enter')
    await expect(dialog).toHaveCount(0)
    await expect(
      page.getByText('Added “Renew the Apple cert” to TODOs'),
    ).toBeVisible()

    await openPalette()
    await page.keyboard.type('todo')
    await page.keyboard.press('Enter')
    await expect(chip).toBeVisible()
    await page.keyboard.type('Second task')
    await page.keyboard.press('Enter')
    await expect(dialog).toHaveCount(0)

    await page
      .getByRole('navigation', { name: 'Harness' })
      .getByRole('button', { name: /TODOs/ })
      .click()
    const rows = page.locator('[data-todo-id]')
    await expect(rows.nth(-2)).toContainText('Renew the Apple cert')
    await expect(rows.last()).toContainText('Second task')

    expect(errors).toEqual([])
  })
})
