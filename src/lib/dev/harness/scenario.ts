import type {
  ApprovalRow,
  EngineStatus,
  FindingRow,
  RepoGitRuntime,
  ResumeThreadSessionInput,
  Snapshot,
  ThreadEventRow,
} from '../../ipc/bindings'
import type { ToastPayload } from '../../feedback/toast-payload'

/**
 * How a browser session of the app should start. Playwright sets it with `setScenario` from
 * `e2e/harness.ts`. The fields marked "URL" also work as query parameters, for `pnpm dev`.
 */
export type HarnessScenario = {
  /** URL `?longThread=N`: N finished turns in the Lead thread (virtualized timeline). */
  longThread?: number
  /** URL `?secondRepo`: another repo, for anything that picks between them. */
  secondRepo?: boolean
  /** URL `?slowReply`: a long reply at a real agent's pace instead of a quick one. */
  slowReply?: boolean
  /** URL `?slowDiff`: holds diff fetches long enough to see the Changes splash. */
  slowDiff?: boolean
  /** URL `?bigDiff=N`: N long TypeScript files, to see how Changes copes with a big diff. */
  bigDiff?: number
  /**
   * URL `?stack=mode`, the gh-stack stand-in. `none` starts unstacked, `unavailable` has no
   * extension, `behind` puts main ahead of the unstacked branch, and `remote` fails to reach
   * GitHub until a test sets `window.__HARNESS__.stackRemoteUp`, then finds the stack there.
   */
  stack?: 'none' | 'unavailable' | 'remote' | 'behind'
  /** A real thread's events (as exported from cormux.db) for the Lead thread. */
  timeline?: ThreadEventRow[]
  /** Approvals to go with `timeline`. */
  approvals?: ApprovalRow[]
  /** Review findings; this makes the fixture workspace a finished review. */
  reviewFindings?: FindingRow[]
  /** Settings rows, as a returning user has them. */
  settings?: Record<string, string>
  /** The fixture repo's run command. */
  runCommand?: string
  /** The usage the Lead's engine reported. */
  leadUsage?: {
    usedTokens: number | null
    contextSize: number | null
    costUsd: number | null
  }
  /** Synced PRs; this also flips GitHub to connected. */
  pullRequests?: Snapshot['persisted']['pullRequests']
  /** A stored ClickUp key and a picked sprint folder. */
  clickup?: boolean
  /** Replaces the detected engines. */
  engines?: EngineStatus[]
  /** Replaces the repos' git state, which is otherwise empty. */
  repoGit?: RepoGitRuntime[]
  /** The workspace whose teardown preview reports nothing to lose. */
  cleanTeardown?: string
  /** Starts the window in full screen. */
  fullscreen?: boolean
}

/** `window.__HARNESS__`: what tests read back from a running session, and how they drive it. */
export type HarnessControls = {
  prompts: {
    threadId: string
    text: string
    attachments: { name: string; dataBase64: string | null }[]
  }[]
  resolved: { id: string; approved: boolean; denyReason: string | null }[]
  answered: { id: string; answers: Record<string, string> }[]
  /** URLs handed to the system browser. */
  opened: string[]
  openedTerminal: string | null
  resumed: ResumeThreadSessionInput | null
  clickupCalls: { cmd: string; args: unknown }[]
  /** `HarnessScenario.stack` of `remote`: whether GitHub answers yet. */
  stackRemoteUp: boolean
  /** Sets a workspace's status without driving an agent. */
  setStatus: (name: string, status: string) => void
  /** Sets a thread's status, such as `running` for its live status, without a prompt. */
  setThreadStatus: (threadId: string, status: string) => void
  /** Delivers events to the open Lead thread as an engine would. */
  agentEvents: (events: object[]) => void
  /** Commits the uncommitted diff, as the watcher reports it. */
  commit: () => void
  /** Raises a toast of any tone. Set just after install: wait for it with `waitForFunction`. */
  showToast?: (payload: ToastPayload) => void
}

declare global {
  interface Window {
    __HARNESS_SCENARIO__?: HarnessScenario
    __HARNESS__?: HarnessControls
  }
}

function scenarioFromUrl(): HarnessScenario {
  const params = new URLSearchParams(window.location.search)
  const scenario: HarnessScenario = {}
  if (params.has('longThread'))
    scenario.longThread = Number(params.get('longThread'))
  if (params.has('secondRepo')) scenario.secondRepo = true
  if (params.has('slowReply')) scenario.slowReply = true
  if (params.has('slowDiff')) scenario.slowDiff = true
  if (params.has('bigDiff')) scenario.bigDiff = Number(params.get('bigDiff'))
  if (params.has('stack'))
    scenario.stack = params.get('stack') as HarnessScenario['stack']
  return scenario
}

export function readScenario(): HarnessScenario {
  return { ...scenarioFromUrl(), ...window.__HARNESS_SCENARIO__ }
}
