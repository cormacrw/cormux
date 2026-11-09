import type { ThreadStatus } from '$lib/ipc/bindings'
import { isAgentWorking } from '$lib/state/activity'
import type { RunningAgent, TimelineItem } from './timeline-types'

/** What the agent is doing now, shown while it works or sits paused mid-turn. */
export type LiveStatus = {
  title: string
  subtitle: string
  paused: boolean
  agents: RunningAgent[]
}

type LiveInput = {
  status: ThreadStatus
  paused: boolean
  activity: string
  /** The workspace's own Lead, which sets up the worktree rather than joining it. */
  lead: boolean
  /** The tool the agent is in, as it last reported. */
  currentTool: string | null
  agents: RunningAgent[]
}

export function liveStatus(input: LiveInput): LiveStatus | null {
  const { status, paused } = input
  if (!isAgentWorking(status) && !(status === 'paused' && paused)) return null
  return {
    title: paused ? 'Paused by you' : input.activity,
    subtitle: liveSubtitle(input),
    paused,
    agents: input.agents,
  }
}

function liveSubtitle(input: LiveInput): string {
  if (input.paused) return 'Resume to let the agent continue.'
  if (input.status === 'provisioning') {
    return input.lead ? 'Setting up the worktree' : 'Joining the worktree'
  }
  return input.currentTool ?? input.activity
}

/** The items still being written while the agent works: whichever one it is in last. */
export function activeIds(items: TimelineItem[], live: boolean) {
  const last = live ? items.at(-1) : undefined
  return {
    reply: last?.kind === 'reply' ? last.id : null,
    thought: last?.kind === 'thought' ? last.id : null,
    run: last?.kind === 'toolRun' ? last.id : null,
  }
}
