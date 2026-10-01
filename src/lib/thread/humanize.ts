const IDENTIFIER = /^[A-Za-z][A-Za-z0-9_-]*$/
const ACRONYM = /^[A-Z0-9]{2,}$/

/**
 * Turns a camel, Pascal, snake or kebab case name into words (`getPRStatus` → "Get PR
 * status"). Anything that isn't a bare identifier, like a sentence or a path, is left alone.
 */
export function humanizeIdentifier(text: string): string {
  if (!IDENTIFIER.test(text)) return text
  const words = text
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2')
    .split(/[\s_-]+/)
    .filter(Boolean)
    .map((word) => (ACRONYM.test(word) ? word : word.toLowerCase()))
  const [first = '', ...rest] = words
  return [first.charAt(0).toUpperCase() + first.slice(1), ...rest].join(' ')
}

/** MCP tools arrive as `mcp__<server>__<tool>`; this splits them, minus claude.ai's prefix. */
export function parseMcpToolName(
  name: string,
): { server: string; action: string } | null {
  const [, rawServer, tool] = /^mcp__(.+?)__(.+)$/.exec(name) ?? []
  if (!rawServer || !tool) return null
  const serverId = rawServer.replace(/^claude_ai_/, '')
  return {
    server: humanizeIdentifier(serverId),
    // `trelloReadCard` on the Trello server reads as "Read card".
    action: humanizeIdentifier(withoutPrefix(tool, serverId)),
  }
}

/** A tool name as words, for labels that name the tool (approvals). */
export function toolDisplayName(name: string): string {
  const mcp = parseMcpToolName(name)
  return mcp ? `${mcp.server}: ${mcp.action}` : humanizeIdentifier(name)
}

function withoutPrefix(name: string, prefix: string): string {
  if (!name.toLowerCase().startsWith(prefix.toLowerCase())) return name
  const rest = name.slice(prefix.length)
  if (!/^([A-Z]|[_-]+[A-Za-z])/.test(rest)) return name
  return rest.replace(/^[_-]+/, '')
}
