<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { shortcutAria, shortcutLabel } from '$lib/keyboard/shortcuts'
  import { settings } from '$lib/settings/store.svelte'
  import { openWorkspaceEditor } from '$lib/workspace/worktree-actions'
  import SquareCode from '@lucide/svelte/icons/square-code'

  let { workspaceId }: { workspaceId: string } = $props()

  // Blank means the core's default.
  const editor = $derived(settings.values.editorApp.trim() || 'Zed')
</script>

<Button
  variant="secondary"
  size="default"
  class="w-8 px-0"
  aria-label="Open in {editor}"
  aria-keyshortcuts={shortcutAria('openEditor')}
  title="Open in {editor} ({shortcutLabel('openEditor')})"
  data-od-id="ws-open-editor"
  onclick={() => void openWorkspaceEditor(workspaceId)}
>
  <SquareCode class="size-4" aria-hidden="true" />
</Button>
