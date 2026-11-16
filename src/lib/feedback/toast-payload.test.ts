import { describe, expect, it } from 'vitest'
import { toastShowsHere } from './toast-payload'

describe('toastShowsHere', () => {
  it('shows a workspace’s toast only while you are on it', () => {
    expect(toastShowsHere('ws-a', 'ws-a')).toBe(true)
    expect(toastShowsHere('ws-a', 'ws-b')).toBe(false)
    expect(toastShowsHere('ws-a', null)).toBe(false)
  })

  it('always shows a toast about no workspace', () => {
    expect(toastShowsHere(null, 'ws-b')).toBe(true)
    expect(toastShowsHere(undefined, null)).toBe(true)
  })
})
