export type BuddyMood = 'running' | 'needsYou' | 'paused' | 'starting' | 'idle'

/** Clay colors. A thread keeps one for its life; only the face follows status. */
const BUDDY_CLAY = [
  '#7CC894',
  '#F5B33C',
  '#F08A6C',
  '#7FB6E6',
  '#A88BE0',
  '#EE8FAA',
] as const

export function buddyColor(threadId: string): string {
  let hash = 0
  for (let i = 0; i < threadId.length; i++) {
    hash = (Math.imul(hash, 31) + threadId.charCodeAt(i)) >>> 0
  }
  return BUDDY_CLAY[hash % BUDDY_CLAY.length] ?? BUDDY_CLAY[0]
}

export function buddyMoodForThread(thread: {
  status: string
  paused: boolean
  pendingApprovals: number
}): BuddyMood {
  if (thread.pendingApprovals > 0) return 'needsYou'
  if (thread.status === 'provisioning' || thread.status === 'creating') {
    return 'starting'
  }
  if (thread.status === 'paused' || thread.paused) return 'paused'
  if (thread.status === 'running') return 'running'
  return 'idle'
}
