import type { TimelineItem } from './timeline-types'

export function announcementForItem(
  item: TimelineItem,
  role: string,
): string | null {
  switch (item.kind) {
    case 'user':
      return null
    case 'thought':
      return `${role}: ${item.text.slice(0, 240)}`
    case 'toolRun': {
      const step = item.steps[item.steps.length - 1]
      if (!step) return null
      if (step.kind === 'edit') return `${role} edited ${step.path}`
      return `${role}: ${step.title}`
    }
    case 'approval':
      return `${role} needs approval: ${item.title}, ${item.what}`
    case 'findings':
      return 'Review findings are ready'
    case 'plan':
      return `${role} proposed a plan`
    case 'live':
      return null
  }
}

export function newestAgentAnnouncement(
  items: TimelineItem[],
  role: string,
): string | null {
  for (let i = items.length - 1; i >= 0; i -= 1) {
    const item = items[i]
    if (!item || item.kind === 'live') continue
    const text = announcementForItem(item, role)
    if (text) return text
  }
  return null
}
