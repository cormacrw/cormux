import type { BranchHold, WorkspaceStack } from '$lib/ipc/bindings'
import { CHANGE, type HarnessRuntime } from '../runtime'
import type { HandlerSlice, Input, Unsupported } from '../types'

export const gitUnsupported = [
  'abort_workspace_git',
  'create_workspace_branch',
  'create_workspace_pull_request',
  'pull_workspace',
] as const satisfies Unsupported

const FULL_STACK = ['feat/oauth-api', 'feat/oauth-login', 'feat/oauth-ui']
const STACK_STATS: Record<string, [number, number, number, number]> = {
  'feat/oauth-api': [212, 38, 4, 6],
  'feat/oauth-login': [96, 12, 2, 3],
  'feat/oauth-ui': [341, 57, 5, 9],
}
// PRs are opened by hand; the bottom branch already has one.
const STACK_PRS: Record<string, number> = { 'feat/oauth-api': 101 }
// Branches origin doesn't have at the same commit, so deleting them asks first.
const UNPUSHED_BRANCHES = ['feat/oauth-api']
// What holds each branch, as the core's `BranchHolds` answers for the fixture repo.
const BRANCH_HOLDS: Record<string, BranchHold> = {
  main: { kind: 'defaultBranch' },
  'feat/oauth-login': { kind: 'checkedOutBy', workspaces: ['OAuth login'] },
}
// Why Add branch can't add, by the checked-out branch, as the core answers for the full stack.
const STACK_ADD_BLOCKS: Record<string, string> = {
  'feat/oauth-api': 'New branches go on top. Check out feat/oauth-ui first.',
  'feat/oauth-login': 'New branches go on top. Check out feat/oauth-ui first.',
}
// Each stacked branch's PR base, as the core drafts it: the branch below it.
const STACK_PR_BASES: Record<string, string> = {
  'feat/oauth-api': 'main',
  'feat/oauth-login': 'feat/oauth-api',
  'feat/oauth-ui': 'feat/oauth-login',
}

export function gitHandlers({
  scenario,
  controls,
  findWorkspace,
  emitStateChanged,
}: HarnessRuntime) {
  const stackMode = scenario.stack
  const stackBranches: string[] = stackMode ? [] : [...FULL_STACK]

  let localBranches = [
    { name: 'main', subject: 'Initial commit', committed: '3 days ago' },
    {
      name: 'feat/oauth-login',
      subject: 'Add the OAuth callback',
      committed: '2 hours ago',
    },
    {
      name: 'feat/colors',
      subject: 'Tune the accent palette',
      committed: 'yesterday',
    },
    {
      name: 'feat/oauth-api',
      subject: 'Add the token endpoint',
      committed: '3 hours ago',
    },
  ]

  const REPO_HEAD = 'main'

  const setFixtureBranch = (workspaceId: string, branch: string) => {
    const { row, record } = findWorkspace(workspaceId)
    if (row) row.branch = branch
    if (record) record.branch = branch
    emitStateChanged(CHANGE.repoGit)
  }

  const currentBranch = (workspaceId: string) =>
    findWorkspace(workspaceId).record?.branch ?? 'main'

  const stackFor = (workspaceId: string): WorkspaceStack => {
    const current = currentBranch(workspaceId)
    const base = {
      workspaceId,
      trunk: 'main',
      currentBranch: current,
      currentNeedsRebase: stackMode === 'behind' || current === 'feat/oauth-ui',
      message: null,
      installCommand: null,
      branches: [],
      addBlock: null,
    }
    if (stackMode === 'unavailable') {
      return {
        ...base,
        status: 'unavailable',
        message: "The gh-stack extension isn't installed.",
        installCommand: 'gh extension install github/gh-stack',
        addBlock: "The gh-stack extension isn't installed.",
      }
    }
    if (stackMode === 'remote' && !stackBranches.includes(current)) {
      if (!controls.stackRemoteUp) {
        return {
          ...base,
          status: 'notStacked',
          message:
            "Looking for this branch's stack on GitHub failed talking to GitHub. Check `gh auth status`, then retry.",
        }
      }
      stackBranches.push(...FULL_STACK)
    }
    if (!stackBranches.includes(current))
      return { ...base, status: 'notStacked' }
    const branches = stackBranches.map((name, index) => {
      const [additions, deletions, commits, files] = STACK_STATS[name] ?? [
        0, 0, 0, 0,
      ]
      const number = STACK_PRS[name]
      return {
        name,
        parent: stackBranches[index - 1] ?? 'main',
        files,
        additions,
        deletions,
        commits,
        current: name === current,
        merged: false,
        queued: false,
        needsRebase: name === 'feat/oauth-ui',
        hold: null,
        pr: number
          ? {
              number,
              url: `https://github.com/acme/my-app/pull/${number}`,
              state: 'open' as const,
            }
          : null,
      }
    })
    return {
      ...base,
      status: 'stacked',
      branches,
      addBlock: STACK_ADD_BLOCKS[current] ?? null,
    }
  }

  return {
    get_workspace_stack: ({ workspaceId }: { workspaceId: string }) =>
      stackFor(workspaceId),
    switch_workspace_branch: ({ input }: Input<'switch_workspace_branch'>) => {
      setFixtureBranch(input.workspaceId, input.branch)
      return null
    },
    add_stack_branch: ({ input }: Input<'add_stack_branch'>) => {
      const refused = scenario.refusedBranches?.[input.branch.trim()]
      if (refused) throw { kind: 'Invalid', message: refused }
      const current = currentBranch(input.workspaceId)
      if (!stackBranches.includes(current) && current !== 'main')
        stackBranches.push(current)
      stackBranches.push(input.branch)
      setFixtureBranch(input.workspaceId, input.branch)
      return null
    },
    push_stack: () => null,
    sync_stack: () => null,
    default_pr_prompt: () => 'Write the description for this pull request.',
    draft_pr_why: ({ workspaceId }: { workspaceId: string }) => {
      const branch = currentBranch(workspaceId)
      const stacked = stackBranches.includes(branch)
      const [added, deleted, , files] = stacked
        ? (STACK_STATS[branch] ?? [0, 0, 0, 0])
        : [42, 7, 0, 3]
      return {
        workspaceId,
        text: 'Lets people sign in with their Google account.',
        fromLlm: false,
        base: stacked
          ? (STACK_PR_BASES[branch] ?? 'main')
          : (findWorkspace(workspaceId).record?.base ?? 'main'),
        files,
        added,
        deleted,
      }
    },
    get_repo_git: () => [],
    list_repo_branches: () => ({
      branches: ['main', 'develop', 'feat/oauth-login'].map((name) => ({
        name,
        hold: BRANCH_HOLDS[name] ?? null,
      })),
    }),
    list_repo_local_branches: () => ({
      head: REPO_HEAD,
      branches: localBranches.map((row) => ({
        ...row,
        hold: BRANCH_HOLDS[row.name] ?? null,
      })),
    }),
    delete_local_branch: ({
      branch: name,
      force,
    }: {
      branch: string
      force: boolean
    }) => {
      if (name === 'main' || name === 'feat/oauth-login') {
        throw { kind: 'Git', message: `${name} is checked out` }
      }
      if (UNPUSHED_BRANCHES.includes(name) && !force) return 'unpushed'
      localBranches = localBranches.filter((row) => row.name !== name)
      return 'deleted'
    },
  } satisfies HandlerSlice
}
