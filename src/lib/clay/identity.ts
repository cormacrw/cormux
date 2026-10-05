import type { StatusDotVariant } from '$lib/sidebar/status'

/** Clay colours a buddy can be. Status lives in its face, so colour is identity. */
export const CLAY_COLORS = [
  '#7CC894', // leaf
  '#B79CE8', // plum
  '#7FB6E6', // pond
  '#F08A6C', // coral
  '#F3A9BB', // blush
  '#F5B33C', // marigold
] as const

export type BuddyFace =
  | 'happy'
  | 'gasp'
  | 'flat'
  | 'calm'
  | 'sleepy'
  | 'glasses'
  | 'none'

/** A stable clay colour for a workspace, scratch, repo or agent, from its id. */
export function clayColor(id: string): string {
  let hash = 0
  for (let i = 0; i < id.length; i++) {
    hash = (hash * 31 + id.charCodeAt(i)) | 0
  }
  return CLAY_COLORS[Math.abs(hash) % CLAY_COLORS.length]!
}

/** The face that carries a status, so colour never carries it alone. */
export function buddyFace(
  variant: StatusDotVariant,
  needsYou = false,
): BuddyFace {
  if (needsYou) return 'gasp'
  switch (variant) {
    case 'running':
      return 'happy'
    case 'provisioning':
      return 'calm'
    case 'paused':
      return 'flat'
    default:
      return 'sleepy'
  }
}
