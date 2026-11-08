<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import ShortcutKbd from '$lib/primitives/ShortcutKbd.svelte'
  import { shortcutAria } from '$lib/keyboard/shortcuts'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import {
    branchPullRequest,
    primaryPrActionLabel,
  } from '$lib/create-pr/pr-header'
  import { prs, shellDialogs } from '$lib/state'
  import { stacks } from '$lib/changes/stacks.svelte'
  import { openExternal } from '$lib/feedback/open-external'
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request'

  let {
    workspace,
  }: {
    workspace: Workspace
  } = $props()

  const reviewSubmitted = $derived(
    workspace.kind === 'review' && workspace.reviewSubmitted,
  )

  const stackPr = $derived(stacks.branchFor(workspace.id, workspace.branch)?.pr)
  // Any PR for the checked-out branch, not just one this workspace opened.
  const branchPr = $derived(
    branchPullRequest({
      items: prs.items,
      repoId: workspace.repoId,
      branch: workspace.branch,
      prNumber: workspace.prNumber,
      prHtmlUrl: workspace.prHtmlUrl,
      stackPr,
    }),
  )
  const prLabel = $derived(
    branchPr ? primaryPrActionLabel(branchPr.synced, branchPr.number) : '',
  )
  const prUrl = $derived(branchPr?.url ?? null)

  const primaryDisabled = $derived(
    workspace.kind === 'review' ? reviewSubmitted : Boolean(branchPr && !prUrl),
  )

  function onKeydown(event: KeyboardEvent) {
    if (!shellDialogs.allows(event, 'primaryAction')) return
    event.preventDefault()
    if (!primaryDisabled) onPrimaryClick()
  }

  function onPrimaryClick() {
    if (workspace.kind === 'review') {
      shellDialogs.open('submitReview', { workspaceId: workspace.id })
      return
    }
    if (branchPr && prUrl) {
      void openExternal(prUrl)
      return
    }
    shellDialogs.open('createPr', { workspaceId: workspace.id })
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if workspace.kind === 'review'}
  <Button
    variant="default"
    size="default"
    class="gap-2 shrink-0 max-md:flex-1"
    disabled={reviewSubmitted}
    aria-keyshortcuts={shortcutAria('primaryAction')}
    data-od-id="ws-submit-review"
    onclick={onPrimaryClick}
  >
    <GitPullRequest class="size-4" aria-hidden="true" />
    Submit review
    <ShortcutKbd id="primaryAction" />
  </Button>
{:else if branchPr}
  <Button
    variant="secondary"
    size="default"
    class="gap-2 shrink-0 max-md:flex-1"
    disabled={!prUrl}
    aria-keyshortcuts={shortcutAria('primaryAction')}
    data-od-id="ws-create-pr"
    onclick={onPrimaryClick}
  >
    <GitPullRequest class="size-4" aria-hidden="true" />
    {prLabel}
    <ShortcutKbd id="primaryAction" />
  </Button>
{/if}
