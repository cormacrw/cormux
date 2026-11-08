import { beforeEach, describe, expect, it, vi } from 'vitest'

const resolveThreadMedia = vi.fn()

vi.mock('$lib/ipc', () => ({ commands: { resolveThreadMedia } }))
vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string) => `asset://${path}`,
}))

const { resolveLocalMedia } = await import('./resolve-media')

const shot = {
  status: 'ok',
  data: {
    kind: 'image',
    name: 'shot.png',
    path: '/media/shot.png',
    previewUrl: null,
  },
}

describe('resolveLocalMedia', () => {
  beforeEach(() => {
    resolveThreadMedia.mockReset()
    vi.spyOn(console, 'warn').mockImplementation(() => {})
  })

  it('resolves a file once, then answers from the cache', async () => {
    resolveThreadMedia.mockResolvedValue(shot)
    const first = await resolveLocalMedia('shot.png', '/wt-a')
    const again = await resolveLocalMedia('shot.png', '/wt-a')
    expect(first).toEqual({
      kind: 'image',
      name: 'shot.png',
      url: 'asset:///media/shot.png',
    })
    expect(again).toBe(first)
    expect(resolveThreadMedia).toHaveBeenCalledTimes(1)
  })

  it('asks again after a failure instead of keeping it for the session', async () => {
    resolveThreadMedia
      .mockResolvedValueOnce({
        status: 'error',
        error: { kind: 'Io', message: 'busy' },
      })
      .mockRejectedValueOnce(new Error('ipc closed'))
      .mockResolvedValueOnce(shot)
    expect(await resolveLocalMedia('late.png', '/wt-b')).toBeNull()
    expect(await resolveLocalMedia('late.png', '/wt-b')).toBeNull()
    expect(await resolveLocalMedia('late.png', '/wt-b')).not.toBeNull()
    expect(resolveThreadMedia).toHaveBeenCalledTimes(3)
    expect(console.warn).toHaveBeenCalledTimes(2)
  })
})
