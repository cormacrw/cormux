import type { TranscriptEvent } from '$lib/ipc/bindings'
import { appendEvent, findThread, findWorkspace, type HarnessDb } from './db'
import {
  LEAD_THREAD_ID,
  leadThreadFixture,
  NO_RUN_COMMAND,
  REPO_ID,
  WORKSPACE_ID,
  workspaceFixture,
} from './fixtures'
import type { HarnessScenario } from './scenario'

const SEEDED_AT = '2026-09-28 12:00:00'

/** As the engine does when it starts talking: the thread now has a session to resume or clear. */
function startLeadSession(db: HarnessDb) {
  findThread(db, LEAD_THREAD_ID)!.sessionId ??= 'session-seeded'
}

function seedLongThread(db: HarnessDb, turns: number) {
  const push = (event: TranscriptEvent) =>
    appendEvent(db, LEAD_THREAD_ID, event, SEEDED_AT)
  startLeadSession(db)
  for (let turn = 1; turn <= turns; turn += 1) {
    push({ type: 'messageChunk', role: 'user', text: `Question ${turn}` })
    push({ type: 'messageChunk', role: 'agent', text: `Answer ${turn}` })
    push({ type: 'turnEnd', stop_reason: 'end_turn', error: null })
  }
  // Switching away from a branch and back writes two notes with the same title.
  const switches = ['feat/test', 'feat/colors', 'feat/test']
  switches.forEach((branch, index) => {
    const previous = switches[index - 1] ?? 'main'
    push({
      type: 'step',
      id: `step-switch-${index}`,
      title: `Switched to \`${branch}\``,
      detail: 'Checked out in this worktree',
      step: {
        kind: 'branchSwitched',
        branch,
        change: { kind: 'checkedOut', previous },
      },
    })
  })
}

function seedReviewFindings(
  db: HarnessDb,
  findings: NonNullable<HarnessScenario['reviewFindings']>,
) {
  const persisted = db.snapshot.persisted
  const { row, record } = findWorkspace(db, WORKSPACE_ID)
  row!.kind = 'review'
  row!.prNumber = 2
  record!.kind = 'review'
  Object.assign(findThread(db, LEAD_THREAD_ID)!, {
    title: 'Reviewer',
    role: 'reviewer',
  })
  persisted.findings = findings
  db.snapshot.workspaceReviewBlocks = [
    {
      workspaceId: WORKSPACE_ID,
      reason:
        'This is a review workspace; its Reviewer already reviews the PR.',
    },
  ]
  persisted.reviews = [
    { workspaceId: WORKSPACE_ID, status: 'ready', submitted: false },
  ]
  // The core records the card in the thread that ran the review.
  appendEvent(
    db,
    LEAD_THREAD_ID,
    {
      type: 'step',
      id: 'step-findings-ready',
      title: `Review found ${findings.length} findings`,
      detail: null,
      step: { kind: 'reviewFindingsReady', count: findings.length },
    },
    SEEDED_AT,
  )
}

function seedLeadUsage(
  db: HarnessDb,
  usage: NonNullable<HarnessScenario['leadUsage']>,
) {
  Object.assign(findThread(db, LEAD_THREAD_ID)!, usage)
  startLeadSession(db)
  // As a real turn does, log the usage it reported, stamped now.
  appendEvent(db, LEAD_THREAD_ID, {
    type: 'usage',
    used_tokens: usage.usedTokens ?? 0,
    context_size: usage.contextSize ?? 0,
    cost_usd: usage.costUsd,
  })
}

/** The repo's run command, and what the core resolves Run to in each of its workspaces. */
function seedRunCommand(db: HarnessDb, command: string) {
  const { snapshot } = db
  const repo = snapshot.persisted.repos.find((row) => row.id === REPO_ID)!
  repo.runCommand = command
  snapshot.repoRunCommands = [{ repoId: REPO_ID, command, source: 'repo' }]
  const ids = snapshot.persisted.workspaces
    .filter((row) => row.repoId === REPO_ID)
    .map((row) => row.id)
  snapshot.workspaceRunCommands = ids.map((workspaceId) => ({
    workspaceId,
    command,
    source: 'repo' as const,
  }))
  snapshot.workspaceRunBlocks = snapshot.workspaceRunBlocks.filter(
    (block) => !ids.includes(block.workspaceId),
  )
}

/** Shapes a session's fresh fixtures to the scenario, before the app first reads them. */
export function seedFixtures(db: HarnessDb, scenario: HarnessScenario) {
  const { snapshot } = db
  const persisted = snapshot.persisted
  const repo = persisted.repos.find((row) => row.id === REPO_ID)!
  if (scenario.longThread && scenario.longThread > 0)
    seedLongThread(db, scenario.longThread)
  if (scenario.secondRepo) {
    persisted.repos.push({
      ...repo,
      id: 'api',
      name: 'api',
      path: '/tmp/cormux-fixture/api',
    })
  }
  if (scenario.timeline) {
    db.timeline = [
      ...db.timeline.filter((row) => row.threadId !== LEAD_THREAD_ID),
      ...scenario.timeline,
    ]
    if (scenario.timeline.length > 0) startLeadSession(db)
  }
  if (scenario.approvals) persisted.approvals = scenario.approvals
  if (scenario.reviewFindings) seedReviewFindings(db, scenario.reviewFindings)
  Object.assign(snapshot.settings, scenario.settings)
  if (scenario.leadUsage) seedLeadUsage(db, scenario.leadUsage)
  if (scenario.entities) persisted.entities = scenario.entities
  if (scenario.pullRequests) {
    persisted.pullRequests = scenario.pullRequests
    snapshot.githubAuthConfigured = true
  }
  if (scenario.secondWorkspace) {
    const { row, record } = workspaceFixture({
      id: 'ws-billing',
      repoId: REPO_ID,
      name: 'Billing fix',
      branch: 'fix/billing',
      base: 'main',
      worktreePath: '/tmp/cormux-fixture/billing',
      createdAtMs: 1,
    })
    persisted.workspaces.push(row)
    snapshot.workspaces.push(record)
    persisted.threads.push(leadThreadFixture('th-billing', 'ws-billing'))
    snapshot.workspaceOwnThreads.push({
      workspaceId: 'ws-billing',
      threadId: 'th-billing',
    })
    snapshot.workspaceRunBlocks.push({
      workspaceId: 'ws-billing',
      reason: NO_RUN_COMMAND,
    })
    snapshot.repoRemovalBlocks = [
      {
        repoId: REPO_ID,
        reason: 'my-app has 2 workspaces. Tear them down first.',
      },
    ]
  }
  if (scenario.runCommand) seedRunCommand(db, scenario.runCommand)
  if (scenario.retiredThread) {
    persisted.threads.unshift({
      ...leadThreadFixture('th-retired-reviewer', WORKSPACE_ID),
      title: 'Reviewer',
      role: 'reviewer',
      retired: true,
    })
  }
  if (scenario.clickup) {
    snapshot.clickupConfigured = true
    Object.assign(snapshot.settings, {
      clickupWorkspaceId: 'team-1',
      clickupSpaceId: 'space-1',
      clickupFolderId: 'folder-sprints',
    })
  }
  Object.assign(snapshot, scenario.decisions)
}
