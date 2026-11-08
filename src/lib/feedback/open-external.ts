import { openUrl } from '@tauri-apps/plugin-opener'
import { toastCoreError } from './wire-feedback'

/** Opens a link in the system browser, with a toast if it won't open. */
export async function openExternal(url: string) {
  try {
    await openUrl(url)
  } catch (error) {
    toastCoreError(error, 'Could not open the link')
  }
}
