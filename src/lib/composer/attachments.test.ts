import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  attachmentFromFile,
  attachmentFromPath,
  isImageName,
  toPromptAttachment,
} from './attachments'

/** The browser's FileReader, which Node lacks: reads any file as `hi`. */
class FakeReader {
  result: string | null = null
  error = null
  onload: (() => void) | null = null
  onerror: (() => void) | null = null
  readAsDataURL(file: File) {
    this.result = `data:${file.type};base64,aGk=`
    this.onload?.()
  }
}

describe('composer attachments', () => {
  afterEach(() => vi.unstubAllGlobals())

  it('tells images from other files by extension', () => {
    expect(isImageName('Shot.PNG')).toBe(true)
    expect(isImageName('photo.jpeg')).toBe(true)
    expect(isImageName('notes.pdf')).toBe(false)
    expect(isImageName('png')).toBe(false)
  })

  it('names a file on disk by the end of its path', () => {
    expect(attachmentFromPath('/Users/me/Desktop/bug.png').name).toBe('bug.png')
    expect(attachmentFromPath('C:\\shots\\bug.png').name).toBe('bug.png')
  })

  it('names a pasted screenshot by when it was pasted, and keeps a real name', async () => {
    vi.stubGlobal('FileReader', FakeReader)
    const now = new Date('2026-10-19T14:03:09Z')
    const pasted = (name: string, type: string) =>
      attachmentFromFile(new File(['hi'], name, { type }), now)
    expect(await pasted('image.png', 'image/png')).toMatchObject({
      name: 'pasted-2026-10-19-14-03-09.png',
      dataBase64: 'aGk=',
      previewUrl: 'data:image/png;base64,aGk=',
    })
    expect((await pasted('', 'image/jpeg')).name).toBe(
      'pasted-2026-10-19-14-03-09.jpg',
    )
    expect(await pasted('report.pdf', 'application/pdf')).toMatchObject({
      name: 'report.pdf',
      previewUrl: undefined,
    })
  })

  it('refuses a pasted file over 20 MB', async () => {
    vi.stubGlobal('FileReader', FakeReader)
    const big = new File([new Uint8Array(21 * 1024 * 1024)], 'huge.mov')
    await expect(attachmentFromFile(big)).rejects.toThrow(
      'huge.mov is over 20 MB',
    )
  })

  it('sends a path for files on disk and contents for pasted ones', () => {
    const picked = attachmentFromPath('/tmp/notes.txt')
    expect(toPromptAttachment(picked)).toEqual({
      name: 'notes.txt',
      path: '/tmp/notes.txt',
      dataBase64: null,
    })
    expect(
      toPromptAttachment({ id: 'a', name: 'shot.png', dataBase64: 'aGk=' }),
    ).toEqual({ name: 'shot.png', path: null, dataBase64: 'aGk=' })
  })
})
