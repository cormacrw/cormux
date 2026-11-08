/** What an attached entity is, judged from its URL, for its icon. */
export type EntityKind = 'pullRequest' | 'issue' | 'ticket' | 'doc' | 'link'

export interface EntityLook {
  kind: EntityKind
  /** Where it lives, under its name: `acme/app`, `linear.app`. */
  source: string
}

const TICKET_HOSTS = [
  'linear.app',
  'app.clickup.com',
  'atlassian.net',
  'trello.com',
  'app.asana.com',
  'app.shortcut.com',
]
const DOC_HOSTS = [
  'docs.google.com',
  'notion.so',
  'www.notion.so',
  'notion.site',
  'confluence',
  'figma.com',
  'www.figma.com',
]

function matchesHost(host: string, patterns: string[]) {
  return patterns.some(
    (pattern) =>
      host === pattern ||
      host.endsWith(`.${pattern}`) ||
      // Self-hosted Confluence and Jira keep the product in the host.
      (!pattern.includes('.') && host.includes(pattern)),
  )
}

export function entityLook(url: string): EntityLook {
  let parsed: URL
  try {
    parsed = new URL(url)
  } catch {
    return { kind: 'link', source: url }
  }
  const host = parsed.hostname.toLowerCase()
  const path = parsed.pathname.split('/').filter(Boolean)
  if (host === 'github.com' || host.endsWith('.github.com')) {
    const repo = path.slice(0, 2).join('/')
    const section = path[2]
    if (section === 'pull' || section === 'pulls')
      return { kind: 'pullRequest', source: repo }
    if (section === 'issues') return { kind: 'issue', source: repo }
    return { kind: 'link', source: repo || host }
  }
  if (host === 'gitlab.com' && path.includes('merge_requests'))
    return { kind: 'pullRequest', source: host }
  if (matchesHost(host, TICKET_HOSTS) || host.includes('jira'))
    return { kind: 'ticket', source: host }
  if (matchesHost(host, DOC_HOSTS)) return { kind: 'doc', source: host }
  return { kind: 'link', source: host.replace(/^www\./, '') }
}
