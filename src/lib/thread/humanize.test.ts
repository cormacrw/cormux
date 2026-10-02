import { describe, expect, it } from 'vitest'
import { humanizeIdentifier, toolDisplayName } from './humanize'

describe('humanizeIdentifier', () => {
  it.each([
    ['trelloReadCard', 'Trello read card'],
    ['TodoWrite', 'Todo write'],
    ['list_pull_requests', 'List pull requests'],
    ['create-issue', 'Create issue'],
    ['getPRStatus', 'Get PR status'],
    ['URL', 'URL'],
    ['Read', 'Read'],
  ])('%s → %s', (input, expected) => {
    expect(humanizeIdentifier(input)).toBe(expected)
  })

  it('leaves sentences and paths alone', () => {
    expect(humanizeIdentifier('Opened PR #12')).toBe('Opened PR #12')
    expect(humanizeIdentifier('src/foo_bar.ts')).toBe('src/foo_bar.ts')
  })
})

describe('toolDisplayName', () => {
  it('names an MCP tool by server and action', () => {
    expect(toolDisplayName('mcp__claude_ai_Trello__trelloReadCard')).toBe(
      'Trello: Read card',
    )
  })

  it('names other tools in words', () => {
    expect(toolDisplayName('WebFetch')).toBe('Web fetch')
  })
})
