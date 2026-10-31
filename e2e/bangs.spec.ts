import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test.describe('bangs', () => {
  test('saved in settings, used in a prompt bar, and started from the palette', async ({
    page,
  }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    mkdirSync('e2e/output', { recursive: true })
    await page.goto('/')

    const nav = page.getByRole('navigation', { name: 'Harness' })
    await nav.getByRole('button', { name: /Settings/ }).click()
    const section = page.locator('[data-od-id="settings-bangs"]')
    await expect(section.getByRole('heading', { name: 'Bangs' })).toBeVisible()

    // A blank name is refused; spaces and the ! are tidied away.
    await section.getByRole('button', { name: 'Add bang' }).click()
    await expect(section.getByText(/Give the bang a name/)).toBeVisible()
    await section.getByLabel('Add a bang').fill('!Fix Tests')
    await section.getByRole('button', { name: 'Add bang' }).click()
    await expect(section.getByText('!fix-tests', { exact: true })).toBeVisible()
    const prompt = section.getByRole('textbox', { name: 'Prompt' })
    await expect(prompt).toBeFocused()
    await expect(section.getByText('Needs a prompt')).toBeVisible()
    await prompt.fill('Run the tests and fix whatever fails.')
    await expect(section.getByText('Needs a prompt')).toHaveCount(0)
    // A name that's taken is refused.
    await section.getByLabel('Add a bang').fill('fix-tests')
    await section.getByRole('button', { name: 'Add bang' }).click()
    await expect(section.getByText('!fix-tests already exists.')).toBeVisible()
    await page.screenshot({ path: 'e2e/output/bangs-settings.png' })

    // In a prompt bar, ! offers it and picking puts the prompt in, unsent.
    await page.getByRole('button', { name: /^OAuth login,/ }).click()
    const composer = page.locator('#composer-input')
    await composer.fill('First, ')
    await composer.pressSequentially('!fix')
    const picker = page.getByRole('listbox', { name: 'Bangs' })
    await expect(picker.getByRole('option')).toHaveCount(1)
    await expect(picker).toContainText('!fix-tests')
    await page.screenshot({ path: 'e2e/output/bangs-composer.png' })
    await composer.press('Enter')
    await expect(composer).toHaveValue(
      'First, Run the tests and fix whatever fails.',
    )
    await expect(picker).toHaveCount(0)

    // From the palette, !name and more words start a thread in the background.
    const dialog = page.getByRole('dialog')
    await page.getByRole('button', { name: /Search or run a command/ }).click()
    const field = dialog.locator('input')
    await page.keyboard.type('!fix')
    await expect(dialog.getByText('!fix-tests')).toBeVisible()
    // Picking it from the list completes it, just as typing it out does.
    await page.keyboard.press('Enter')
    await expect(field).toHaveValue('!fix-tests ')
    await expect(
      dialog.locator('[data-od-id="palette-bang-start"]'),
    ).toBeVisible()
    // Tab completes a partial name too.
    await field.fill('!fi')
    await page.keyboard.press('Tab')
    await expect(field).toHaveValue('!fix-tests ')
    await page.keyboard.type('only the api')
    await expect(
      dialog.getByText(
        'Run the tests and fix whatever fails. + “only the api”',
      ),
    ).toBeVisible()
    await page.screenshot({ path: 'e2e/output/bangs-palette.png' })
    await page.keyboard.press('Enter')
    await expect(dialog).toHaveCount(0)

    await page
      .locator('[data-od-id="thread-list"]')
      .getByRole('button', { name: /^Run the tests/ })
      .click()
    await expect(
      page.locator('#timeline').getByText(/only the api/),
    ).toBeVisible()

    expect(errors).toEqual([])
  })

  test('scratch macros carry over as bangs', async ({ page }) => {
    await page.addInitScript(() => {
      ;(window as { __HARNESS_SETTINGS__?: unknown }).__HARNESS_SETTINGS__ = {
        scratchMacros: JSON.stringify([
          { id: 'm1', name: 'Morning triage', prompt: 'Check CI' },
        ]),
      }
    })
    await page.goto('/')
    await page
      .getByRole('navigation', { name: 'Harness' })
      .getByRole('button', { name: /Settings/ })
      .click()
    await expect(
      page
        .locator('[data-od-id="settings-bangs"]')
        .getByText('!morning-triage', { exact: true }),
    ).toBeVisible()
  })
})
