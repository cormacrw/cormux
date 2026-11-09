import { describe, expect, it } from 'vitest'
import { mediaKindForSource } from './media'

describe('mediaKindForSource', () => {
  it('classifies by extension, ignoring case and query', () => {
    expect(mediaKindForSource('/a/B.PNG')).toBe('image')
    expect(mediaKindForSource('clip.webm?t=3')).toBe('video')
    expect(mediaKindForSource('/a/readme.md')).toBeNull()
    expect(mediaKindForSource('https://example.com/a.png')).toBeNull()
  })
})
