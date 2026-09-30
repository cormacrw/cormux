import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test.describe('scratches', () => {
  test('starts, opens, messages and deletes a scratch', async ({ page }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    mkdirSync('e2e/output', { recursive: true })
    await page.goto('/')

    const section = page.locator('[data-od-id="home-sessions"]')
    await expect(
      section.getByRole('heading', { name: /Scratches/ }),
    ).toContainText('1')
    const card = section.getByRole('button', {
      name: 'Why the webhook signature fails, my-app, Idle',
    })
    await expect(card).toBeVisible()
    await expect(
      section.getByRole('button', {
        name: 'Delete Why the webhook signature fails',
      }),
    ).toBeVisible()
    // New scratch sits left of New Workspace.
    const newScratch = page.getByRole('button', { name: /New scratch/ })
    const newWorkspace = page.getByRole('button', { name: /New Workspace/ })
    expect((await newScratch.boundingBox())!.x).toBeLessThan(
      (await newWorkspace.boundingBox())!.x,
    )
    await page.screenshot({
      path: 'e2e/output/scratches-home.png',
      fullPage: true,
    })

    // An existing scratch: engine-only speaker line, read-only chip, no workspace chrome.
    await card.click()
    const view = page.locator('#view-session')
    await expect(
      view.getByRole('heading', { name: 'Why the webhook signature fails' }),
    ).toBeFocused()
    await expect(view.getByText('/tmp/cormux-fixture/my-app')).toBeVisible()
    await expect(view.getByText('Read-only, no worktree')).toBeVisible()
    await expect(view.getByText('Searched for constructEvent')).toBeVisible()
    // Thinking stays folded until asked for.
    const thinking = view.getByText(/Signature failures usually mean/)
    await expect(thinking).toBeHidden()
    await view.getByText('Thought', { exact: true }).click()
    await expect(thinking).toBeVisible()
    await expect(
      view.getByRole('button', { name: 'End scratch' }),
    ).toBeVisible()
    await expect(view.getByRole('button', { name: 'Changes' })).toHaveCount(0)
    await expect(
      page.getByRole('textbox', { name: 'Message this scratch' }),
    ).toHaveAttribute('placeholder', 'Ask a follow-up…')
    // The harness runs on Vite's dev server, so this is a dev build.
    await expect(page).toHaveTitle(
      'Cormux Dev · Why the webhook signature fails',
    )
    await expect(
      page.getByRole('button', { name: 'Homebase' }),
    ).not.toHaveAttribute('aria-current', 'page')
    await page.screenshot({
      path: 'e2e/output/scratch-page.png',
      fullPage: true,
    })

    // ⌘S opens the dialog from anywhere, even inside the composer.
    await page.getByRole('textbox', { name: 'Message this scratch' }).focus()
    await page.keyboard.press('ControlOrMeta+s')
    const dialog = page.getByRole('dialog', { name: 'New scratch' })
    await expect(dialog).toBeVisible()
    await expect(dialog.getByRole('textbox', { name: 'Prompt' })).toBeFocused()

    // The title is optional: left blank, the core drafts one from the prompt.
    await dialog
      .getByRole('textbox', { name: 'Prompt' })
      .fill('draft a short changelog for september')
    await expect(dialog.getByRole('textbox', { name: /Title/ })).toHaveValue('')
    await page.screenshot({ path: 'e2e/output/scratch-dialog.png' })
    await dialog
      .getByRole('textbox', { name: 'Prompt' })
      .press('ControlOrMeta+Enter')

    await expect(dialog).toBeHidden()
    await expect(
      view.getByRole('heading', {
        name: 'Draft a short changelog for september',
      }),
    ).toBeVisible()
    await expect(view.getByText('Opened my-app')).toBeVisible()
    await expect(page.getByText('Started scratch')).toBeVisible()
    await expect(view.getByRole('button', { name: 'Pause' })).toBeVisible()

    // A blank start focuses the composer.
    await page.keyboard.press('ControlOrMeta+s')
    await dialog.getByRole('textbox', { name: /Title/ }).fill('Blank scratch')
    await dialog.getByRole('textbox', { name: /Title/ }).press('Enter')
    const composer = page.getByRole('textbox', { name: 'Message this scratch' })
    await expect(composer).toBeFocused()
    await expect(composer).toHaveAttribute(
      'placeholder',
      'Write the first message…',
    )
    await expect(
      page.getByRole('button', { name: 'Send', exact: true }),
    ).toBeDisabled()
    await composer.fill('where are sessions stored?')
    await composer.press('Enter')
    await expect(view.getByText('where are sessions stored?')).toBeVisible()
    await expect(composer).toHaveAttribute('placeholder', 'Ask a follow-up…')

    // End from the page goes home.
    await view.getByRole('button', { name: 'End scratch' }).click()
    const confirm = page.getByRole('alertdialog')
    await expect(confirm.getByText('End this scratch?')).toBeVisible()
    await confirm.getByRole('button', { name: 'End scratch' }).click()
    await expect(page.getByRole('heading', { name: 'Homebase' })).toBeFocused()
    await expect(page.getByText('Ended Blank scratch')).toBeVisible()

    // Delete from a card keeps Homebase and focuses the next card.
    await expect(
      section.getByRole('heading', { name: /Scratches/ }),
    ).toContainText('2')
    await section
      .getByRole('button', {
        name: 'Delete Draft a short changelog for september',
      })
      .click()
    await expect(confirm.getByText('Delete this scratch?')).toBeVisible()
    await expect(confirm.getByText('/tmp/cormux-fixture/my-app')).toBeVisible()
    await confirm.getByRole('button', { name: 'Delete scratch' }).click()
    await expect(
      page.getByText('Deleted Draft a short changelog for september'),
    ).toBeVisible()
    await expect(
      section.getByRole('button', {
        name: 'Why the webhook signature fails, my-app, Idle',
      }),
    ).toBeFocused()

    // Palette lists New scratch and each scratch.
    await page.keyboard.press('ControlOrMeta+k')
    await expect(
      page.getByRole('option', { name: /New scratch/ }),
    ).toBeVisible()
    await expect(
      page.getByRole('option', {
        name: /Open Why the webhook signature fails/,
      }),
    ).toBeVisible()

    expect(errors).toEqual([])
  })
})
