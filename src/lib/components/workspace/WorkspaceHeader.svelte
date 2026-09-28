<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import * as Tooltip from '$lib/components/ui/tooltip'
  import { isWorkspaceProvisioning } from '$lib/workspace/provisioning'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import WorkspaceMoreMenu from './WorkspaceMoreMenu.svelte'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import Play from '@lucide/svelte/icons/play'

  let {
    workspace,
    titleRef = $bindable(),
  }: {
    workspace: Workspace
    titleRef?: HTMLHeadingElement | undefined
  } = $props()

  const provisioning = $derived(isWorkspaceProvisioning(workspace.lifecycle))
  const runDisabled = $derived(
    provisioning || workspace.lifecycle === 'provisioningFailed',
  )
</script>

<header
  class="flex flex-wrap items-start justify-between gap-3 border-b border-border/60 pb-4"
>
  <div class="min-w-0 grid gap-1">
    <h1
      bind:this={titleRef}
      tabindex="-1"
      class="truncate text-xl font-semibold tracking-tight outline-none"
    >
      {workspace.name}
    </h1>
    <p class="text-sm text-muted-foreground">{workspace.activityText}</p>
  </div>

  <div class="flex flex-wrap items-center gap-2">
    <Tooltip.Provider>
      <Tooltip.Root>
        <Tooltip.Trigger>
          {#snippet child({ props })}
            <Button {...props} size="sm" disabled={runDisabled} class="gap-2">
              {#if provisioning}
                <LoaderCircle
                  class="size-4 {runDisabled ? 'animate-spin' : ''}"
                  aria-hidden="true"
                />
              {:else}
                <Play class="size-4" aria-hidden="true" />
              {/if}
              Run
            </Button>
          {/snippet}
        </Tooltip.Trigger>
        {#if runDisabled}
          <Tooltip.Content>Available once the worktree is set up</Tooltip.Content>
        {/if}
      </Tooltip.Root>
    </Tooltip.Provider>
    <WorkspaceMoreMenu workspaceId={workspace.id} />
  </div>
</header>
