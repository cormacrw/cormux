<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import {
    primaryPrActionLabel,
    pullRequestForWorkspace,
    resolvePrHtmlUrl,
  } from '$lib/create-pr/pr-header'
  import { prs, workspaceRecords } from '$lib/state'
  import { openUrl } from '@tauri-apps/plugin-opener'
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

  const record = $derived(workspaceRecords.getRecord(workspace.id))
  const linkedPr = $derived(
    record
      ? pullRequestForWorkspace(prs.items, record.repoId, workspace.prNumber)
      : undefined,
  )
  const prLabel = $derived(
    workspace.prNumber != null
      ? primaryPrActionLabel(linkedPr, workspace.prNumber)
      : 'Create PR',
  )
  const prUrl = $derived(
    resolvePrHtmlUrl(linkedPr, workspace.prHtmlUrl),
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
    if (workspace.prNumber != null && prUrl) {
      void openUrl(prUrl)
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
    disabled={!prUrl}
    data-ws-focus="primary"
    data-od-id="ws-create-pr"
    onclick={onPrimaryClick}
  >
    <GitPullRequest class="size-4" aria-hidden="true" />
    {prLabel}
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
