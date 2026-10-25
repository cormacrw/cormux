import { groupWorkspacesByRepo } from '$lib/sidebar/workspace-groups'
import { repos } from './repos.svelte'
import { workspaceRecords } from './workspace-records.svelte'
import { workspaces } from './workspaces.svelte'

/**
 * The sidebar's workspaces, grouped by repo. `ordered` is the order they show
 * in, which ⌘1–⌘9 and the Ctrl+Tab list follow too.
 */
class SidebarWorkspaces {
  readonly groups = $derived(
    groupWorkspacesByRepo(
      workspaces.sidebarItems,
      (id) => workspaceRecords.getRecord(id)?.repoId,
      repos.items,
    ),
  )
  readonly ordered = $derived(this.groups.flatMap((group) => group.workspaces))
}

export const sidebarWorkspaces = new SidebarWorkspaces()
