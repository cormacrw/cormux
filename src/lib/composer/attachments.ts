import type { PromptAttachment } from '$lib/ipc/bindings'

/** An image or file waiting in the composer until the message is sent. */
export type ComposerAttachment = {
  id: string
  name: string
  /** A file on disk (picked or dropped from Finder). */
  path?: string
  /** A pasted or browser-dropped file's contents, which has no path. */
  dataBase64?: string
  /** A thumbnail for images we have the bytes of. */
  previewUrl?: string
}

/** Pasted and dropped files travel to the core whole, so very large ones are refused. */
export const MAX_ATTACHMENT_BYTES = 20 * 1024 * 1024

const IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp']

export function isImageName(name: string): boolean {
  const extension = name.split('.').pop()?.toLowerCase() ?? ''
  return name.includes('.') && IMAGE_EXTENSIONS.includes(extension)
}

/** The last part of a path, for a file picked or dropped by path. */
export function fileNameOf(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path
}

/** A pasted screenshot arrives as `image.png`; give it a name that says when. */
export function pastedName(name: string, type: string, now: Date): string {
  if (name && name !== 'image.png') return name
  const extension = type.split('/')[1]?.replace('jpeg', 'jpg') || 'png'
  const stamp = now.toISOString().slice(0, 19).replace(/[T:]/g, '-')
  return `pasted-${stamp}.${extension}`
}

/** The message the conversation shows; the core saves the same text. */
export function shownMessage(text: string, names: string[]): string {
  if (!names.length) return text
  const line = `Attached: ${names.join(', ')}`
  return text ? `${text}\n\n${line}` : line
}

/** What the core needs to turn an attachment into a file the agent can open. */
export function toPromptAttachment(
  attachment: ComposerAttachment,
): PromptAttachment {
  return {
    name: attachment.name,
    path: attachment.path ?? null,
    dataBase64: attachment.dataBase64 ?? null,
  }
}

/** Reads a pasted or dropped file, with a thumbnail for images. */
export async function attachmentFromFile(
  file: File,
  now = new Date(),
): Promise<ComposerAttachment> {
  if (file.size > MAX_ATTACHMENT_BYTES) {
    throw new Error(`${file.name || 'That file'} is over 20 MB`)
  }
  const dataUrl = await new Promise<string>((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(String(reader.result))
    reader.onerror = () => reject(reader.error ?? new Error('Could not read'))
    reader.readAsDataURL(file)
  })
  const name = pastedName(file.name, file.type, now)
  return {
    id: crypto.randomUUID(),
    name,
    dataBase64: dataUrl.slice(dataUrl.indexOf(',') + 1),
    previewUrl: isImageName(name) ? dataUrl : undefined,
  }
}

export function attachmentFromPath(path: string): ComposerAttachment {
  return { id: crypto.randomUUID(), name: fileNameOf(path), path }
}
