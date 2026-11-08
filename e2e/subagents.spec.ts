import { expect, test } from '@playwright/test'
import { mkdirSync } from 'node:fs'

const call = (
  id: string,
  name: string,
  title: string,
  detail: string,
  parent_id: string | null = null,
) => ({
  type: 'toolCall',
  id,
  title,
  role:
    name === 'Agent'
      ? { kind: 'subagent', description: detail }
      : { kind: 'read' },
  status: 'inProgress',
  locations: [],
  detail,
  parent_id,
})

const done = (id: string) => ({
  type: 'toolCallUpdate',
  id,
  title: null,
  status: 'completed',
  locations: [],
})

test('running subagents show in the thread until they report back', async ({
  page,
}) => {
  const errors: string[] = []
  page.on('pageerror', (error) => errors.push(error.message))
  await page.goto('/?longThread=1')
  await page.getByRole('button', { name: /^OAuth login,/ }).click()
  const timeline = page.locator('#timeline')
  await expect(timeline.getByText('Answer 1')).toBeVisible()

  await page.evaluate(
    (events) => {
      window.__HARNESS__!.setThreadStatus('th-lead', 'running')
      window.__HARNESS__!.agentEvents(events)
    },
    [
      { type: 'messageChunk', role: 'user', text: 'Audit the auth flow' },
      call('a1', 'Agent', 'Agent', 'Map the token refresh path'),
      call('a2', 'Agent', 'Agent', 'Find callers of useSession'),
      call('c1', 'Read', 'Read', 'src/lib/auth.ts', 'a1'),
      call('c2', 'Grep', 'Grep', 'refreshToken', 'a1'),
      call('c3', 'Glob', 'Glob', 'src/**/*.svelte', 'a2'),
    ],
  )

  const running = page.locator('[data-od-id="running-agents"]')
  await expect(running.getByRole('listitem')).toHaveCount(2)
  await expect(running).toContainText('Map the token refresh path')
  await expect(running).toContainText('Grep refreshToken')
  await expect(
    page.locator('[data-od-id="live-status"]').getByText('· 2 agents running'),
  ).toBeVisible()
  await expect(
    timeline.locator('[data-od-id="agent-state"]').first(),
  ).toContainText('running · 2 steps')
  mkdirSync('e2e/output', { recursive: true })
  await page.screenshot({
    path: 'e2e/output/subagents-running.png',
    animations: 'disabled',
  })

  await page.evaluate(
    (events) => window.__HARNESS__!.agentEvents(events),
    [done('a1')],
  )
  await expect(running.getByRole('listitem')).toHaveCount(1)
  await expect(
    page.locator('[data-od-id="live-status"]').getByText('· 1 agent running'),
  ).toBeVisible()

  await page.evaluate(
    (events) => window.__HARNESS__!.agentEvents(events),
    [done('a2')],
  )
  await expect(running).toHaveCount(0)
  expect(errors).toEqual([])
})
