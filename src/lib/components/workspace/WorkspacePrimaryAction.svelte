<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request'

  let {
    workspace,
  }: {
    workspace: Workspace
  } = $props()

  const reviewSubmitted = $derived(
    workspace.kind === 'review' &&
      workspace.activityText.toLowerCase().includes('review submitted'),
  )

  function onPrimaryClick() {
    if (workspace.kind === 'review') {
      window.dispatchEvent(
        new CustomEvent('cormux:submit-review', {
          detail: { workspaceId: workspace.id },
        }),
      )
      return
    }
    window.dispatchEvent(
      new CustomEvent('cormux:create-pr', {
        detail: { workspaceId: workspace.id },
      }),
    )
  }
</script>

{#if workspace.kind === 'review'}
  <Button
    variant="default"
    size="sm"
    class="gap-2 shrink-0 max-md:flex-1"
    disabled={reviewSubmitted}
    data-ws-focus="primary"
    data-od-id="ws-submit-review"
    onclick={onPrimaryClick}
  >
    <GitPullRequest class="size-4" aria-hidden="true" />
    Submit review
  </Button>
{:else if workspace.prNumber}
  <Button
    variant="default"
    size="sm"
    class="gap-2 shrink-0 max-md:flex-1"
    disabled
    data-ws-focus="primary"
    data-od-id="ws-create-pr"
  >
    <GitPullRequest class="size-4" aria-hidden="true" />
    PR #{workspace.prNumber} opened
  </Button>
{:else}
  <Button
    variant="default"
    size="sm"
    class="gap-2 shrink-0 max-md:flex-1"
    data-ws-focus="primary"
    data-od-id="ws-create-pr"
    onclick={onPrimaryClick}
  >
    <GitPullRequest class="size-4" aria-hidden="true" />
    Create PR
  </Button>
{/if}
