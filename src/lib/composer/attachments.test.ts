import { describe, expect, it } from 'vitest'
import {
  attachmentFromPath,
  fileNameOf,
  isImageName,
  pastedName,
  toPromptAttachment,
} from './attachments'

describe('composer attachments', () => {
  it('tells images from other files by extension', () => {
    expect(isImageName('Shot.PNG')).toBe(true)
    expect(isImageName('photo.jpeg')).toBe(true)
    expect(isImageName('notes.pdf')).toBe(false)
    expect(isImageName('png')).toBe(false)
  })

  it('names a file by the end of its path', () => {
    expect(fileNameOf('/Users/me/Desktop/bug.png')).toBe('bug.png')
    expect(fileNameOf('C:\\shots\\bug.png')).toBe('bug.png')
  })

  it('names a pasted screenshot by when it was pasted', () => {
    const now = new Date('2026-10-19T14:03:09Z')
    expect(pastedName('image.png', 'image/png', now)).toBe(
      'pasted-2026-10-19-14-03-09.png',
    )
    expect(pastedName('', 'image/jpeg', now)).toBe(
      'pasted-2026-10-19-14-03-09.jpg',
    )
    expect(pastedName('report.pdf', 'application/pdf', now)).toBe('report.pdf')
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
