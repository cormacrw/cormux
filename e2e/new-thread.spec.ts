import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

test('the first page is a new thread, and typing into it spawns a workspace', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1280, height: 800 })
  await page.goto('/')
  await expect(page.getByRole('heading', { name: 'New thread' })).toBeVisible()
  await expect(page.getByRole('dialog')).toHaveCount(0)
  await expect(page.locator('[data-od-id="thread-tabs"]')).toHaveCount(0)
  const composer = page.locator('#composer-input')
  await expect(composer).toBeFocused()

  // The repo and base live in the prompt bar, the base as a search.
  await expect(page.locator('#new-thread-repo')).toHaveText('my-app')
  const base = page.locator('#new-thread-base')
  await expect(base).toContainText('main')
  await base.click()
  await expect(page.getByPlaceholder('Search branches…')).toBeFocused()
  await page.keyboard.type('dev')
  await page.keyboard.press('Enter')
  await expect(base).toContainText('develop')

  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/new-thread.png' })

  await composer.fill('Add rate limiting to the API')
  await page.keyboard.press('Enter')

  // Straight into the thread, with the message showing.
  await expect(page.locator('[data-od-id="new-thread"]')).toHaveCount(0)
  await expect(
    page.locator('#timeline').getByText('Add rate limiting to the API'),
  ).toBeVisible()
  await expect(
    page.locator('[data-od-id="thread-list"] [aria-current="page"]'),
  ).toContainText('my-app')
})

test('⌘↵ starts a thread and stays on the new thread page', async ({
  page,
}) => {
  await page.goto('/')
  const composer = page.locator('#composer-input')
  await composer.fill('Queue this one up')
  await page.keyboard.press('ControlOrMeta+Enter')

  const list = page.locator('[data-od-id="thread-list"]')
  await expect(list.getByText('Queue this one up')).toBeVisible()
  await expect(page.locator('[data-od-id="new-thread"]')).toBeVisible()
  await expect(composer).toHaveValue('')
  await expect(composer).toBeFocused()
})

test('the new thread page offers skills and hints at them and bangs', async ({
  page,
}) => {
  await page.goto('/')
  const foot = page.locator('[data-od-id="composer-foot"]')
  await expect(foot).toContainText('skills')
  await expect(foot).toContainText('bangs')
  await expect(foot).not.toContainText('send')
  await expect(foot).not.toContainText('new line')

  await page.locator('#composer-input').pressSequentially('/')
  const options = page.getByRole('listbox', { name: 'Skills' })
  await expect(options.getByText('/deploy')).toBeVisible()
  await page.keyboard.press('Enter')
  await expect(page.locator('#composer-input')).toHaveValue(/^\/deploy/)
})

test('going from a new thread into it plays the first prompt transition', async ({
  page,
}) => {
  await page.goto('/')
  await page.evaluate(() => {
    const seen: string[] = []
    ;(window as unknown as { __CLASSES__: string[] }).__CLASSES__ = seen
    new MutationObserver(() =>
      seen.push(document.documentElement.className),
    ).observe(document.documentElement, { attributes: true })
  })
  await page.locator('#composer-input').fill('Fall away')
  await page.keyboard.press('Enter')
  await expect(page.locator('[data-od-id="new-thread"]')).toHaveCount(0)
  const classes = await page.evaluate(
    () => (window as unknown as { __CLASSES__: string[] }).__CLASSES__,
  )
  expect(classes.join(' ')).toContain('first-prompt')
})

test('a new thread starts on the model picked in its prompt bar', async ({
  page,
}) => {
  await page.goto('/')
  await page.getByRole('button', { name: 'Model: Default' }).click()
  await page.getByRole('menuitemradio', { name: 'Haiku' }).click()
  await expect(page.locator('#composer-input')).toBeFocused()
  await expect(page.getByRole('button', { name: 'Model: Haiku' })).toBeVisible()

  await page.locator('#composer-input').fill('Pick a model first')
  await page.keyboard.press('Enter')
  await expect(page.locator('[data-od-id="new-thread"]')).toHaveCount(0)
  await expect(page.getByRole('button', { name: 'Model: Haiku' })).toBeVisible()
})

test('sending a message turns the row into a spinner straight away', async ({
  page,
}) => {
  await page.goto('/')
  const row = page.getByRole('button', { name: /^OAuth login,/ })
  await row.click()
  await expect(row).toHaveAttribute('data-urgency', 'attention')

  await page.locator('#composer-input').fill('Do the thing')
  await page.keyboard.press('Enter')
  // The mock never moves the workspace on, so this is the thread alone.
  await expect(row.locator('[data-od-id="thread-row-spinner"]')).toBeVisible()
  await expect(row).toHaveAttribute('data-urgency', 'quiet')
  await expect(row).toHaveAttribute('data-urgency', 'attention', {
    timeout: 20_000,
  })
})

test('the sidebar is a flat list that says where each thread lives', async ({
  page,
}) => {
  await page.goto('/')
  const row = page.getByRole('button', { name: /^OAuth login,/ })
  await expect(row).toContainText('my-app')
  await expect(row).toContainText('feat/')
  await expect(page.locator('#side-repo-label')).toHaveCount(0)
})

test('a working thread only moves to Working once you leave it', async ({
  page,
}) => {
  await page.goto('/')
  const threads = page.locator('[data-od-id="thread-list"]')
  const toggle = page.locator('[data-od-id="working-toggle"]')
  await threads.getByRole('button', { name: /^OAuth login,/ }).click()
  await page.evaluate(() =>
    (
      window as unknown as {
        __HARNESS_SET_STATUS__: (name: string, status: string) => void
      }
    ).__HARNESS_SET_STATUS__('OAuth login', 'running'),
  )

  const row = page.getByRole('button', { name: /^OAuth login,/ })
  await expect(threads.locator(row)).toHaveAttribute('data-urgency', 'quiet')
  await expect(toggle).toHaveCount(0)

  await page.locator('[data-od-id="nav-new-thread"]').click()
  await expect(toggle).toContainText('1')
  await expect(toggle).toHaveAttribute('aria-expanded', 'false')
  await expect(row).toHaveCount(0)

  await toggle.click()
  await expect(
    page.locator('[data-od-id="working-list"]').locator(row),
  ).toBeVisible()

  // Opening it brings it back up, so the thread you're on is always in sight.
  await page.locator('[data-od-id="working-list"]').locator(row).click()
  await expect(threads.locator(row)).toBeVisible()
  await expect(toggle).toHaveCount(0)
})

test('a thread flashes unless it is working, and carries no status text', async ({
  page,
}) => {
  await page.goto('/')
  await page.locator('[data-od-id="nav-new-thread"]').click()
  await page.locator('#composer-input').fill('Second thread')
  await page.keyboard.press('Enter')
  await expect(page.locator('[data-od-id="new-thread"]')).toHaveCount(0)

  const row = page.getByRole('button', { name: /^OAuth login,/ })
  await expect(row).toHaveAttribute('data-urgency', 'attention')
  await expect(row).not.toContainText('Idle')

  const rows = page.locator('[data-od-id="thread-list"] button')
  await expect(rows).toHaveCount(3)
  const setStatus = (name: string, status: string) =>
    page.evaluate(
      ([name, status]) =>
        (
          window as unknown as {
            __HARNESS_SET_STATUS__: (name: string, status: string) => void
          }
        ).__HARNESS_SET_STATUS__(name!, status!),
      [name, status],
    )

  // Working goes quiet and moves below what's waiting, with no words either way.
  await setStatus('OAuth login', 'running')
  await expect(row).toHaveCount(0)
  await page.locator('[data-od-id="working-toggle"]').click()
  await expect(row).toHaveAttribute('data-urgency', 'quiet')
  await expect(row.locator('[data-od-id="thread-row-spinner"]')).toBeVisible()
  await expect(row.locator('.opacity-50')).toBeVisible()
  await expect(
    row.locator('.opacity-50 .text-muted-foreground').first(),
  ).toContainText('OAuth login')
  mkdirSync('e2e/output', { recursive: true })
  await page.waitForTimeout(1600) // let the rows and the greeting settle
  await page.screenshot({ path: 'e2e/output/thread-working.png' })
  await expect(row).not.toContainText('Running')
  await expect(rows.first()).toContainText('Second thread')

  await setStatus('OAuth login', 'waiting')
  await expect(row).toHaveAttribute('data-urgency', 'attention')
  // The dot really blinks: sampled across a cycle, it's seen both full and dimmed.
  const opacities = await row.locator('.status-flash').evaluate(
    (dot) =>
      new Promise<number[]>((resolve) => {
        const seen: number[] = []
        const timer = setInterval(() => {
          seen.push(Number(getComputedStyle(dot).opacity))
          if (seen.length === 30) {
            clearInterval(timer)
            resolve(seen)
          }
        }, 100)
      }),
  )
  expect(Math.max(...opacities)).toBeGreaterThan(0.9)
  expect(Math.min(...opacities)).toBeLessThan(0.7)
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({ path: 'e2e/output/thread-attention.png' })
})
