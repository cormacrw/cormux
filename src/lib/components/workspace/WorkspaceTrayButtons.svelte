<script lang="ts">
  import ChangeCounts from '$lib/components/workspace/ChangeCounts.svelte'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import { workspaceUi } from '$lib/state'
  import { findings } from '$lib/findings/store.svelte'
  import { workspaceDiff } from '$lib/changes/diff-store.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import { cn } from '$lib/utils'
  import { shortcutAria, shortcutLabel } from '$lib/keyboard/shortcuts'
  import { formatChangeCounts } from '$lib/changes/diff-totals'
  import { appStatusView } from '$lib/run-app/status'
  import {
    changesTabAriaLabel,
    findingsTabAriaLabel,
    outputTabAriaLabel,
  } from '$lib/workspace/thread-tabs'
  import GitBranch from '@lucide/svelte/icons/git-branch'
  import List from '@lucide/svelte/icons/list'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import Terminal from '@lucide/svelte/icons/terminal'

  let { workspace }: { workspace: Workspace } = $props()

  const showFindings = $derived(findings.tabAvailable(workspace))
  const openFindings = $derived(findings.openCount(workspace.id))
  const app = $derived(appStatusView(workspace))
  const changeCountLabel = $derived(
    formatChangeCounts(workspaceDiff.uncommitted(workspace.id)),
  )

  function buttonClass(open: boolean) {
    return cn(
      'h-8 gap-1.5 rounded-[9px] px-2.5 font-normal',
      open
        ? 'bg-[color-mix(in_oklch,var(--foreground)_12%,transparent)] font-medium text-foreground'
        : 'text-muted-foreground hover:text-foreground',
    )
  }
</script>

<div class="flex items-center gap-1" data-od-id="ws-trays">
  {#if showFindings}
    {@const open = workspaceUi.activeTab === 'findings'}
    <Button
      variant="ghost"
      size="sm"
      class={buttonClass(open)}
      aria-pressed={open}
      aria-label={findingsTabAriaLabel(openFindings)}
      id="tray-findings"
      data-od-id="tray-findings"
      onclick={() => workspaceUi.toggleTray('findings')}
    >
      <List class="size-3.5 shrink-0" aria-hidden="true" />
      <span aria-hidden="true">Findings</span>
      {#if openFindings > 0}
        <Badge
          variant="secondary"
          class="min-w-[18px] justify-center px-1 font-mono text-[10px]"
          aria-hidden="true"
        >
          {openFindings}
        </Badge>
      {/if}
    </Button>
  {/if}

  <Button
    variant="ghost"
    size="sm"
    class={buttonClass(workspaceUi.gitVisible)}
    aria-pressed={workspaceUi.gitVisible}
    aria-label={changesTabAriaLabel(changeCountLabel)}
    aria-keyshortcuts={shortcutAria('toggleGit')}
    title="Git ({shortcutLabel('toggleGit')})"
    id="tray-changes"
    data-od-id="tray-changes"
    onclick={() => workspaceUi.toggleTray('changes')}
  >
    <GitBranch class="size-3.5 shrink-0" aria-hidden="true" />
    <span aria-hidden="true">Git</span>
    {#key workspace.id}
      <ChangeCounts totals={workspaceDiff.uncommitted(workspace.id)} />
    {/key}
  </Button>

  <Button
    variant="ghost"
    size="sm"
    class={buttonClass(workspaceUi.outputVisible)}
    aria-pressed={workspaceUi.outputVisible}
    aria-label={outputTabAriaLabel(app)}
    aria-keyshortcuts={shortcutAria('toggleOutput')}
    title="Output ({shortcutLabel('toggleOutput')})"
    id="tray-output"
    data-od-id="tray-output"
    onclick={() => workspaceUi.toggleTray('output')}
  >
    <Terminal class="size-3.5 shrink-0" aria-hidden="true" />
    <span aria-hidden="true">Output</span>
    {#if app.busy}
      <LoaderCircle class="size-3.5 animate-spin" aria-hidden="true" />
    {:else if app.port}
      <span class="flex items-center gap-1" aria-hidden="true">
        <span class="size-2 rounded-full bg-emerald-500"></span>
        <span class="font-mono text-[10px] text-muted-foreground"
          >:{app.port}</span
        >
      </span>
    {/if}
  </Button>
</div>
