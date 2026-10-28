import { describe, expect, it } from 'vitest'
import { mediaKindForSource, splitLocalMedia } from './local-media'

describe('splitLocalMedia', () => {
  it('pulls a local screenshot out of the prose, in place', () => {
    const segments = splitLocalMedia(
      'Here is the result:\n\n![Settings after the fix](/tmp/shots/settings.png)\n\nLooks right.',
    )
    expect(segments).toEqual([
      { kind: 'text', text: 'Here is the result:\n\n' },
      {
        kind: 'media',
        source: '/tmp/shots/settings.png',
        alt: 'Settings after the fix',
        mediaKind: 'image',
      },
      { kind: 'text', text: '\n\nLooks right.' },
    ])
  })

  it('treats a link to a local recording as a video', () => {
    expect(splitLocalMedia('[Watch the flow](demo.webm)')).toEqual([
      {
        kind: 'media',
        source: 'demo.webm',
        alt: 'Watch the flow',
        mediaKind: 'video',
      },
    ])
  })

  it('accepts file URLs and angle-bracket paths with spaces', () => {
    expect(
      splitLocalMedia('![a](file:///tmp/my%20shots/one.png)'),
    ).toContainEqual({
      kind: 'media',
      source: 'file:///tmp/my%20shots/one.png',
      alt: 'a',
      mediaKind: 'image',
    })
    expect(splitLocalMedia('![b](</tmp/my shots/two.mp4>)')).toContainEqual({
      kind: 'media',
      source: '/tmp/my shots/two.mp4',
      alt: 'b',
      mediaKind: 'video',
    })
  })

  it('accepts raw img and video tags pointing at local files', () => {
    expect(
      splitLocalMedia(
        '<img src="/tmp/a.png" alt="A"> and <video src="/tmp/b.mov"></video>',
      ),
    ).toEqual([
      { kind: 'media', source: '/tmp/a.png', alt: 'A', mediaKind: 'image' },
      { kind: 'text', text: ' and ' },
      { kind: 'media', source: '/tmp/b.mov', alt: '', mediaKind: 'video' },
    ])
  })

  it('leaves remote images, document links and code fences alone', () => {
    const text = [
      '![remote](https://example.com/a.png)',
      '[notes](/tmp/notes.pdf)',
      '```md',
      '![example](/tmp/example.png)',
      '```',
    ].join('\n')
    expect(splitLocalMedia(text)).toEqual([{ kind: 'text', text }])
  })
})

describe('mediaKindForSource', () => {
  it('classifies by extension, ignoring case and query', () => {
    expect(mediaKindForSource('/a/B.PNG')).toBe('image')
    expect(mediaKindForSource('clip.webm?t=3')).toBe('video')
    expect(mediaKindForSource('/a/readme.md')).toBeNull()
    expect(mediaKindForSource('https://example.com/a.png')).toBeNull()
  })
})
