import { describe, expect, it } from 'vitest'
import { applyDraftUpdate, draftText } from './drafts'

describe('composer drafts', () => {
  it('keeps separate drafts per thread', () => {
    let store: Record<string, string> = {}
    store = applyDraftUpdate(store, 'a', 'hello lead')
    store = applyDraftUpdate(store, 'b', 'hello reviewer')
    expect(draftText(store, 'a')).toBe('hello lead')
    expect(draftText(store, 'b')).toBe('hello reviewer')
    store = applyDraftUpdate(store, 'a', '')
    expect(draftText(store, 'a')).toBe('')
    expect(draftText(store, 'b')).toBe('hello reviewer')
  })
})
