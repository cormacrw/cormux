import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test.describe('scratch macros', () => {
  test('saves a macro and runs it from the palette in the background', async ({
    page,
  }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    mkdirSync('e2e/output', { recursive: true })
    await page.goto('/')

    const nav = page.getByRole('navigation', { name: 'Harness' })
    await nav.getByRole('button', { name: /Settings/ }).click()
    const section = page.locator('[data-od-id="settings-macros"]')
    await expect(
      section.getByRole('heading', { name: 'Scratch macros' }),
    ).toBeVisible()

    // A blank name is refused.
    await section.getByRole('button', { name: 'Add macro' }).click()
    await expect(section.getByText('Give the macro a name.')).toBeVisible()

    await section.getByRole('textbox').first().fill('Morning triage')
    await section.getByRole('button', { name: 'Add macro' }).click()
    const prompt = section.getByRole('textbox', { name: 'Prompt' })
    await expect(prompt).toBeFocused()
    await expect(section.getByText('Not in the palette')).toBeVisible()
    await prompt.fill('Check what failed in CI overnight')
    await expect(section.getByText('Not in the palette')).toHaveCount(0)
    await page.screenshot({ path: 'e2e/output/scratch-macros-settings.png' })

    const dialog = page.getByRole('dialog')
    const field = dialog.locator('input')
    const chip = page.locator('[data-od-id="palette-macro-chip"]')
    const openPalette = async () => {
      await page
        .getByRole('button', { name: /Search or run a command/ })
        .click()
      await expect(field).toBeFocused()
    }

    // The list shows the prompt's first 20 characters under the name.
    await openPalette()
    await page.keyboard.type('Morning')
    await expect(dialog.getByText('Check what failed in…')).toBeVisible()
    await page.keyboard.press('Enter')
    await expect(dialog).toHaveCount(0)

    // Stays on Settings; the toast confirms.
    await expect(page.getByText('Started Morning triage')).toBeVisible()
    await expect(section).toBeVisible()

    // The full name then Space turns into a chip; Backspace turns it back.
    await openPalette()
    await page.keyboard.type('morning triage ')
    await expect(chip).toContainText('Morning triage')
    await expect(field).toHaveValue('')
    await page.keyboard.press('Backspace')
    await expect(chip).toHaveCount(0)
    await expect(field).toHaveValue('Morning triage')

    // Tab works too, and what follows the chip joins the prompt.
    await page.keyboard.press('Tab')
    await expect(chip).toBeVisible()
    await page.keyboard.type('only the web job')
    await expect(
      dialog.getByText('Check what failed in… + “only the web job”'),
    ).toBeVisible()
    await page.screenshot({ path: 'e2e/output/scratch-macros-palette.png' })
    await page.keyboard.press('Enter')
    await expect(dialog).toHaveCount(0)
    await expect(section).toBeVisible()

    const toast = page.getByText('Started Morning triage').first()
    await expect(toast).toBeVisible()
    await toast.click()
    const view = page.locator('#view-session')
    await expect(
      view.getByRole('heading', { name: 'Morning triage' }),
    ).toBeVisible()
    await expect(view.getByText(/only the web job/)).toBeVisible()

    expect(errors).toEqual([])
  })
})
