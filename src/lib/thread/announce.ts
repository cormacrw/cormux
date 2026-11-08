import type { TimelineItem } from './timeline-types'
import { stepText, workedForLabel } from './timeline/step-text'

function announcementForItem(item: TimelineItem, role: string): string | null {
  switch (item.kind) {
    case 'user':
      return null
    case 'reply':
    case 'thought':
      return `${role}: ${item.text.slice(0, 240)}`
    case 'toolRun': {
      const step = item.steps.at(-1)
      if (!step) return null
      if (step.kind === 'tool' && step.role.kind === 'edit') {
        return `${role} edited ${step.locations[0] ?? ''}`
      }
      return `${role}: ${stepText(step).title}`
    }
    case 'worked':
      return `${role}: ${workedForLabel(item.startedAtMs, item.atMs)}`
    case 'event':
      return item.title.replaceAll('`', '')
    case 'approval':
      return `${role} needs approval: ${item.title}, ${item.what}`
    case 'findings':
      return 'Review findings are ready'
    case 'plan':
      return `${role} proposed a plan`
  }
}

export function newestAgentAnnouncement(
  items: TimelineItem[],
  role: string,
): string | null {
  for (let i = items.length - 1; i >= 0; i -= 1) {
    const text = announcementForItem(items[i]!, role)
    if (text) return text
  }
  return null
}
