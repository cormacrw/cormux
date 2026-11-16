import type { Window } from '@tauri-apps/api/window'
import { workspaces } from '$lib/state/workspaces.svelte'
import { stacks } from './stacks.svelte'
import { workspacePr } from './workspace-pr'

// The synced PR list already refreshes this often in the core; this keeps merged and queued up.
const POLL_MS = 120_000

/**
 * Keeps the sidebar's PR icons current. Whether a PR merged or joined the merge queue comes from
 * its workspace's stack, which is otherwise only read when you open the workspace. So every
 * couple of minutes, and whenever the window regains focus, each workspace with a PR still in
 * flight reads its stack again, one at a time and quietly. Returns the uninstall.
 */
export async function installPrRefresh(win: Window): Promise<() => void> {
  let running = false
  const refresh = async () => {
    if (running) return
    running = true
    try {
      for (const workspace of workspaces.liveItems) {
        const pr = workspacePr(workspace)
        if (!pr || pr.state === 'merged') continue
        await stacks.load(workspace.id, { quiet: true })
      }
    } finally {
      running = false
    }
  }
  const unlistenFocus = await win.onFocusChanged(({ payload: focused }) => {
    if (focused) void refresh()
  })
  const poll = window.setInterval(() => void refresh(), POLL_MS)
  return () => {
    unlistenFocus()
    window.clearInterval(poll)
  }
}
