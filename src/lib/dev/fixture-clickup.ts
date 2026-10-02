import type {
  ClickupBoard,
  ClickupTask,
  ClickupTaskDetail,
  ClickupUser,
} from '$lib/ipc/bindings'

/** Placeholder people and tasks for the browser harness; not real ClickUp data. */
const me: ClickupUser = {
  id: 101,
  username: 'You',
  initials: 'YO',
  color: '#7b68ee',
}
const teammate: ClickupUser = {
  id: 102,
  username: 'Robin Park',
  initials: 'RP',
  color: '#0ab4a8',
}

function task(
  id: string,
  customId: string | null,
  name: string,
  status: string,
  statusColor: string,
  points: number | null,
  assignees: ClickupUser[],
): ClickupTask {
  return {
    id,
    customId,
    name,
    status,
    statusColor,
    points,
    assignees,
    priority: id === 't2' ? 'high' : null,
    priorityColor: id === 't2' ? '#f8ae00' : null,
    parent: null,
    url: `https://app.clickup.com/t/${id}`,
  }
}

const DAY = 86_400_000

export function fixtureClickupBoard(): ClickupBoard {
  const now = Date.now()
  return {
    sprint: {
      id: 'list-sprint-14',
      name: 'Sprint 14',
      startMs: now - 6 * DAY,
      dueMs: now + 7.5 * DAY,
    },
    statuses: [
      { name: 'to do', color: '#87909e', kind: 'open', order: 0 },
      { name: 'in progress', color: '#4194f6', kind: 'custom', order: 1 },
      { name: 'in review', color: '#a875ff', kind: 'custom', order: 2 },
      { name: 'complete', color: '#008844', kind: 'closed', order: 3 },
    ],
    tasks: [
      task(
        't1',
        'ENG-201',
        'Rate limit the public search endpoint',
        'to do',
        '#87909e',
        3,
        [teammate],
      ),
      task(
        't2',
        'ENG-198',
        'Session expires mid checkout on Safari',
        'in progress',
        '#4194f6',
        null,
        [me],
      ),
      task(
        't3',
        'ENG-190',
        'Move invoice PDFs to the background queue',
        'in progress',
        '#4194f6',
        5,
        [me],
      ),
      task(
        't4',
        // Workspaces without Custom Task IDs only have ClickUp's own.
        null,
        'Audit log for admin role changes',
        'to do',
        '#87909e',
        null,
        [],
      ),
      task(
        't5',
        'ENG-184',
        'Upgrade Postgres driver',
        'in review',
        '#a875ff',
        2,
        [teammate],
      ),
      task(
        't6',
        'ENG-177',
        'Onboarding checklist copy',
        'complete',
        '#008844',
        1,
        [me],
      ),
    ],
    userId: me.id,
  }
}

export function fixtureClickupDetail(source: ClickupTask): ClickupTaskDetail {
  return {
    task: source,
    description: [
      '## Problem',
      '',
      'Customers lose their cart when the session cookie expires **mid checkout**.',
      '',
      '### Steps',
      '',
      '1. Add an item',
      '2. Wait 30 minutes',
      '3. Check out',
      '',
      '| Browser | Affected |',
      '| --- | --- |',
      '| Safari 18 | Yes |',
      '| Chrome | ~~No~~ Sometimes |',
      '',
      '- [x] Reproduce locally',
      '- [ ] Extend the cookie on activity',
      '',
      'See [the incident](https://example.com/incident).',
    ].join('\n'),
    tags: [{ name: 'checkout', fg: '#ffffff', bg: '#e5484d' }],
    creator: teammate,
    listName: 'Sprint 14',
    dueMs: null,
    createdMs: Date.now() - 3 * DAY,
    updatedMs: Date.now() - DAY,
  }
}
