<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import {
    branchPullRequest,
    primaryPrActionLabel,
  } from '$lib/create-pr/pr-header'
  import { reviewSubmitted as isReviewSubmitted } from '$lib/review/workspace-settings'
  import {
    prs,
    settings,
    shellDialogs,
    stacks,
    workspaceRecords,
  } from '$lib/state'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request'

  let {
    workspace,
  }: {
    workspace: Workspace
  } = $props()

  const reviewSubmitted = $derived(
    workspace.kind === 'review' &&
      (workspace.activityText.toLowerCase().includes('review submitted') ||
        isReviewSubmitted(settings.rows, workspace.id)),
  )

  const record = $derived(workspaceRecords.getRecord(workspace.id))
  const stackPr = $derived(
    stacks
      .get(workspace.id)
      ?.branches.find((branch) => branch.name === workspace.branch)?.pr,
  )
  // Any PR for the checked-out branch, not just one this workspace opened.
  const branchPr = $derived(
    branchPullRequest({
      items: prs.items,
      repoId: record?.repoId ?? '',
      branch: workspace.branch,
      prNumber: workspace.prNumber,
      prHtmlUrl: workspace.prHtmlUrl,
      stackPr,
    }),
  )
  const prLabel = $derived(
    branchPr
      ? primaryPrActionLabel(branchPr.synced, branchPr.number)
      : 'Create PR',
  )
  const prUrl = $derived(branchPr?.url ?? null)

  function onPrimaryClick() {
    if (workspace.kind === 'review') {
      shellDialogs.openSubmitReview(workspace.id)
      return
    }
    if (branchPr && prUrl) {
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
{:else if branchPr}
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
