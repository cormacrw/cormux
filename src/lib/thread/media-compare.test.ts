import { describe, expect, it } from 'vitest'
import { splitLocalMedia } from './local-media'
import { pairBeforeAfter } from './media-compare'

describe('pairBeforeAfter', () => {
  it('pairs images captioned before and after, across a blank line', () => {
    const segments = splitLocalMedia(
      '![Before](/a.png)\n\n![After the fix](/b.png)',
    )
    expect(pairBeforeAfter(segments)).toEqual([
      { kind: 'compare', before: '/a.png', after: '/b.png' },
    ])
  })

  it('leaves other images alone', () => {
    const segments = splitLocalMedia('![one](/a.png)\n\n![two](/b.png)')
    expect(pairBeforeAfter(segments)).toEqual(segments)
  })
})
