import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test.describe('repos', () => {
  test('lists repos above workspaces, pulls the default branch, and sets it per repo', async ({
    page,
  }) => {
    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    mkdirSync('e2e/output', { recursive: true })
    await page.goto('/')

    const nav = page.getByRole('navigation', { name: 'Harness' })
    const repoList = nav.getByRole('list', { name: 'Repos' })
    const workspaceList = nav.getByRole('list', { name: /Workspaces/ })
    await expect(repoList.getByText('my-app')).toBeVisible()
    expect((await repoList.boundingBox())!.y).toBeLessThan(
      (await workspaceList.boundingBox())!.y,
    )

    const pull = repoList.getByRole('button', { name: 'Pull main for my-app' })
    await pull.click()
    await expect(pull).toBeDisabled()
    await expect(pull).toBeEnabled()
    await page.screenshot({ path: 'e2e/output/repos-sidebar.png' })

    await page.getByRole('button', { name: 'Settings' }).click()
    await expect(page.getByText('Default base branch')).toHaveCount(0)
    await page.getByRole('button', { name: 'Configure my-app' }).click()
    const field = page.getByRole('button', { name: 'Default branch' })
    await expect(field).toHaveText('main')
    await field.click()
    await page.getByRole('option', { name: 'develop' }).click()
    await expect(field).toHaveText('develop')
    await expect(
      repoList.getByRole('button', { name: 'Pull develop for my-app' }),
    ).toBeVisible()
    await page.screenshot({ path: 'e2e/output/repos-settings.png' })

    expect(errors).toEqual([])
  })
})
