<script lang="ts">
  import { pluralWord } from '$lib/format/plural'
  import { tick } from 'svelte'
  import { openExternal } from '$lib/feedback/open-external'
  import GitBranch from '@lucide/svelte/icons/git-branch'
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request'
  import Layers from '@lucide/svelte/icons/layers'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import MessageSquare from '@lucide/svelte/icons/message-square'
  import Plus from '@lucide/svelte/icons/plus'
  import RefreshCw from '@lucide/svelte/icons/refresh-cw'
  import Upload from '@lucide/svelte/icons/upload'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import { commands } from '$lib/ipc'
  import { diffComments } from '$lib/changes/diff-comments.svelte'
  import type { DiffTarget } from '$lib/ipc/bindings'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { prs, repos } from '$lib/state'
  import { stacks } from '$lib/changes/stacks.svelte'
  import { loadStack } from '$lib/changes/actions'
  import { workspaceDiff } from '$lib/changes/diff-store.svelte'
  import { shortcutLabel } from '$lib/keyboard/shortcuts'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import {
    canAddToStack,
    commitsLabel,
    isDiffTargetOf,
    sameDiffTarget,
    stackCardsTopFirst,
    stackSubtitle,
  } from '$lib/changes/stack'
  import {
    addStackBranch,
    pushStack,
    switchWorkspaceBranch,
    syncStack,
  } from '$lib/workspace/git-actions'
  import { cn } from '$lib/utils'
  import { branchHoldLabel } from '$lib/repo/branch-hold'
  import { startsOver } from '$lib/core/owner-scoped'

  let { workspace, repoId }: { workspace: Workspace; repoId: string } = $props()

  const stack = $derived(stacks.get(workspace.id))
  const cards = $derived(stackCardsTopFirst(stack))
  const trunk = $derived(stack?.trunk ?? repos.defaultBranch(repoId))
  const stacked = $derived(stack?.status === 'stacked')
  const checking = $derived(stacks.loading[workspace.id] ?? false)
  const lockReason = $derived(workspace.branchLock)
  const locked = $derived(lockReason != null)
  const addable = $derived(canAddToStack(stack))
  const unavailable = $derived({
    reason: stack?.message ?? '',
    command: stack?.installCommand ?? null,
  })
  const topBranch = $derived(cards[0]?.name)
  const addTitle = $derived(
    locked
      ? lockReason
      : !addable && topBranch
        ? `New branches go on top. Check out ${topBranch} first.`
        : `Create a branch on top of ${workspace.branch}`,
  )

  // While a retarget loads, the highlight moves to where it is headed.
  const shownTarget = $derived(workspaceDiff.shownTarget(workspace.id))
  // Without a stack the checked-out branch still gets a level of its own.
  const soloBranch = $derived(!!stack && !stacked && workspace.branch !== trunk)

  // The tray stays mounted across workspaces; what's under way belongs to the one it started in.
  const workspaceId = $derived(workspace.id)
  let busy = $derived<'push' | 'sync' | null>(startsOver(workspaceId, null))
  let switchingTo = $derived<string | null>(startsOver(workspaceId, null))
  let adding = $derived(startsOver(workspaceId, false))
  let creating = $derived(startsOver(workspaceId, false))
  let newBranch = $derived(startsOver(workspaceId, ''))
  let addError = $derived<string | null>(startsOver(workspaceId, null))
  let addInput = $state<HTMLInputElement | null>(null)

  async function select(next: DiffTarget | null) {
    if (sameDiffTarget(next, shownTarget)) return
    const id = workspaceId
    await workspaceDiff.fetch(
      id,
      async () => {
        const result = await commands.setWorkspaceDiffTarget(id, next)
        if (result.status === 'error') toastCoreError(result.error)
        return result.status === 'ok'
      },
      { target: next },
    )
  }

  function selected(branch: string, base: string) {
    return isDiffTargetOf(shownTarget, branch, base, workspace.branch)
  }

  /** A PR opened by hand shows up here before gh-stack learns of it on the next Sync. */
  function syncedPr(branch: string) {
    const pr = prs.forBranch(repoId, branch)
    return pr
      ? { number: pr.num, url: pr.htmlUrl, state: 'open' as const }
      : null
  }

  async function checkout(name: string) {
    if (locked || switchingTo) return
    const id = workspaceId
    switchingTo = name
    await switchWorkspaceBranch(id, name)
    if (id === workspaceId) switchingTo = null
  }

  async function runRemote(action: 'push' | 'sync') {
    if (busy) return
    const id = workspaceId
    busy = action
    const ok = await (action === 'push' ? pushStack : syncStack)(id)
    if (id === workspaceId) busy = null
    // A branch switch reloads the stack on its own; rebases and pushes don't.
    if (ok) await loadStack(id)
  }

  async function openAdd() {
    adding = true
    addError = null
    await tick()
    addInput?.focus()
  }

  function closeAdd() {
    adding = false
    newBranch = ''
    addError = null
  }

  async function submitAdd() {
    const id = workspaceId
    creating = true
    const { added, refused } = await addStackBranch(id, newBranch)
    if (id !== workspaceId) return
    creating = false
    if (added) closeAdd()
    else addError = refused
  }

  const levelClass = (active: boolean) =>
    cn(
      'flex min-w-0 flex-1 items-center gap-1 rounded-lg border transition-colors',
      active
        ? 'border-primary/50 bg-primary/10'
        : 'border-border/70 hover:bg-muted/40',
    )
  const levelButtonClass =
    'min-w-0 flex-1 rounded-lg px-2.5 py-2 text-left outline-none focus-visible:ring-2 focus-visible:ring-ring/50 disabled:cursor-default'
</script>

{#snippet needsRebase(title: string)}
  <Badge
    variant="outline"
    class="shrink-0 border-warning/40 bg-warning/10 px-1.5 text-[10px] text-warning"
    {title}>needs rebase</Badge
  >
{/snippet}

{#snippet commentCount(branch: string | null)}
  {@const count = diffComments.forBranch(workspace.id, branch).length}
  {#if count}
    <Badge
      variant="outline"
      class="shrink-0 gap-0.5 border-info/40 bg-info/10 px-1.5 font-mono text-[10px] text-info"
      title="{count} unsent {count === 1
        ? 'comment'
        : 'comments'} on this level"
    >
      <MessageSquare class="size-2.5" aria-hidden="true" />
      {count}
      <span class="sr-only">{pluralWord(count, 'comment')}</span>
    </Badge>
  {/if}
{/snippet}

{#snippet dot(kind: 'current' | 'merged' | 'open' | 'uncommitted')}
  <div class="flex w-4 shrink-0 justify-center" aria-hidden="true">
    <span
      class={cn(
        'mt-3.5 size-2.5 shrink-0 rounded-full',
        kind === 'current' && 'bg-primary ring-4 ring-primary/20',
        kind === 'merged' && 'bg-muted-foreground/50',
        kind === 'open' &&
          'border-2 border-muted-foreground/50 bg-background-solid',
        kind === 'uncommitted' &&
          'border-2 border-dashed border-muted-foreground/60 bg-background-solid',
      )}
    ></span>
  </div>
{/snippet}

<nav
  aria-label="Stack"
  data-od-id="stack"
  class="flex min-h-0 w-72 shrink-0 flex-col border-r border-border/60"
>
  <header class="shrink-0 space-y-2 border-b border-border/60 px-3 py-2.5">
    <div class="min-w-0">
      <h2 class="text-sm font-semibold">Stack</h2>
      <p class="text-xs text-muted-foreground">
        {stack ? stackSubtitle(stack) : 'Reading the stack…'}
      </p>
    </div>
    {#if stack && stack.status !== 'unavailable'}
      <div class="flex flex-wrap items-center gap-1">
        {#if stacked}
          <Button
            size="xs"
            variant="ghost"
            disabled={busy != null || locked}
            title={locked
              ? lockReason
              : 'Fetch, rebase onto the trunk, push, and refresh pull requests'}
            data-od-id="stack-sync"
            onclick={() => void runRemote('sync')}
          >
            {#if busy === 'sync'}
              <LoaderCircle class="animate-spin" aria-hidden="true" />
            {:else}
              <RefreshCw aria-hidden="true" />
            {/if}
            Sync
          </Button>
        {:else}
          <Button
            size="xs"
            variant="ghost"
            disabled={checking}
            title="Look for a stack on GitHub that has {workspace.branch}"
            data-od-id="stack-check"
            onclick={() => void loadStack(workspace.id)}
          >
            {#if checking}
              <LoaderCircle class="animate-spin" aria-hidden="true" />
            {:else}
              <RefreshCw aria-hidden="true" />
            {/if}
            Check GitHub
          </Button>
        {/if}
        {#if !adding}
          <Button
            size="xs"
            variant={stacked ? 'ghost' : 'outline'}
            disabled={locked || !addable || busy != null}
            title={addTitle}
            data-od-id="stack-add"
            onclick={() => void openAdd()}
          >
            <Plus aria-hidden="true" />
            Add branch
          </Button>
        {/if}
        {#if stacked}
          <Button
            size="xs"
            class="ml-auto"
            disabled={busy != null}
            title="Push every open branch in the stack to origin. Pull requests stay manual."
            data-od-id="stack-push"
            onclick={() => void runRemote('push')}
          >
            {#if busy === 'push'}
              <LoaderCircle class="animate-spin" aria-hidden="true" />
            {:else}
              <Upload aria-hidden="true" />
            {/if}
            Push
          </Button>
        {/if}
      </div>
    {/if}
  </header>

  <div class="min-h-0 flex-1 overflow-y-auto px-3 pt-3 pb-6">
    {#if stack?.status === 'notStacked' && stack.message}
      <p
        class="mb-3 rounded-lg border border-border/70 bg-muted/20 p-3 text-xs"
        data-od-id="stack-check-failed"
      >
        {stack.message}
      </p>
    {/if}

    <div class="relative">
      <span
        class="absolute top-5 bottom-3 left-[7.5px] w-px bg-border"
        aria-hidden="true"
      ></span>
      <ol class="relative" aria-label="Stack levels, top first">
        <li class="relative flex gap-2 pb-2" data-stack-uncommitted>
          {@render dot('uncommitted')}
          <div class={levelClass(shownTarget === null)}>
            <button
              type="button"
              class={levelButtonClass}
              aria-pressed={shownTarget === null}
              onclick={() => void select(null)}
            >
              <span class="flex items-center gap-1.5">
                <span class="truncate text-sm">Uncommitted changes</span>
                {@render commentCount(null)}
              </span>
              <span class="block truncate text-xs text-muted-foreground">
                Not committed to {workspace.branch} yet
              </span>
            </button>
          </div>
        </li>

        {#if adding}
          <li class="relative flex gap-2 pb-2">
            {@render dot('uncommitted')}
            <form
              class="min-w-0 flex-1 rounded-lg border border-dashed border-border bg-muted/20 p-2.5"
              data-od-id="stack-add-form"
              onsubmit={(event) => {
                event.preventDefault()
                void submitAdd()
              }}
            >
              <label
                for="stack-new-branch"
                class="text-xs text-muted-foreground"
              >
                New branch on top of <code class="font-mono text-foreground"
                  >{workspace.branch}</code
                >
              </label>
              <input
                bind:this={addInput}
                id="stack-new-branch"
                bind:value={newBranch}
                placeholder="feat/next-step"
                autocomplete="off"
                spellcheck="false"
                aria-invalid={addError != null}
                aria-describedby="stack-add-hint"
                class="mt-2 w-full rounded-md border border-input bg-background px-2 py-1 font-mono text-xs outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40"
                oninput={() => (addError = null)}
                onkeydown={(event) => {
                  if (event.key === 'Escape') {
                    event.stopPropagation()
                    closeAdd()
                  }
                }}
              />
              <p
                id="stack-add-hint"
                class={cn(
                  'mt-1.5 text-xs',
                  addError ? 'text-destructive' : 'text-muted-foreground',
                )}
              >
                {addError ?? 'Uncommitted changes move to the new branch.'}
              </p>
              <div class="mt-2 flex justify-end gap-1.5">
                <Button
                  type="button"
                  size="xs"
                  variant="ghost"
                  onclick={closeAdd}
                >
                  Cancel
                </Button>
                <Button
                  type="submit"
                  size="xs"
                  variant="secondary"
                  disabled={creating || !newBranch.trim()}
                >
                  {#if creating}
                    <LoaderCircle class="animate-spin" aria-hidden="true" />
                  {/if}
                  Create
                </Button>
              </div>
            </form>
          </li>
        {/if}

        {#if soloBranch}
          {@const active = selected(workspace.branch, trunk)}
          <li
            class="relative flex gap-2 pb-2"
            data-stack-branch={workspace.branch}
          >
            {@render dot('current')}
            <div class={levelClass(active)}>
              <button
                type="button"
                class={levelButtonClass}
                aria-pressed={active}
                title="Committed changes on {workspace.branch} since it left {trunk}"
                onclick={() =>
                  void select({ head: workspace.branch, base: trunk })}
              >
                <span class="flex flex-wrap items-center gap-x-1.5 gap-y-1">
                  <span class="max-w-full truncate font-mono text-sm"
                    >{workspace.branch}</span
                  >
                  {@render commentCount(workspace.branch)}
                  {#if stack?.status === 'notStacked'}
                    <Badge
                      variant="secondary"
                      class="shrink-0 px-1.5 text-[10px]">not stacked</Badge
                    >
                  {/if}
                  {#if stack?.currentNeedsRebase}
                    {@render needsRebase(
                      `${trunk} moved on since ${workspace.branch} left it. Pull it in from the command palette (${shortcutLabel('palette')}) to catch up.`,
                    )}
                  {/if}
                </span>
                <span class="block text-xs text-muted-foreground"
                  >vs {trunk}</span
                >
              </button>
            </div>
          </li>
        {/if}

        {#each cards as card (card.name)}
          {@const switching = switchingTo === card.name}
          {@const pr = card.pr ?? syncedPr(card.name)}
          {@const held = card.hold}
          {@const active = selected(card.name, card.parent)}
          <li class="relative flex gap-2 pb-2" data-stack-branch={card.name}>
            {@render dot(
              card.current ? 'current' : card.merged ? 'merged' : 'open',
            )}
            <div
              class={cn(levelClass(active), card.merged && 'opacity-60')}
              aria-current={card.current ? 'true' : undefined}
            >
              <button
                type="button"
                class={levelButtonClass}
                aria-pressed={active}
                disabled={card.merged}
                title={card.merged
                  ? `${card.name} was merged into ${trunk}`
                  : `Committed changes on ${card.name} since it left ${card.parent}`}
                onclick={() =>
                  void select({ head: card.name, base: card.parent })}
              >
                <span class="flex flex-wrap items-center gap-x-1.5 gap-y-1">
                  <span class="max-w-full truncate font-mono text-sm"
                    >{card.name}</span
                  >
                  {@render commentCount(card.name)}
                  {#if card.current}
                    <Badge
                      variant="secondary"
                      class="shrink-0 px-1.5 text-[10px]">current</Badge
                    >
                  {/if}
                  {#if card.needsRebase}
                    {@render needsRebase(
                      `${card.parent} moved on since this branch was based on it. Sync to rebase.`,
                    )}
                  {/if}
                </span>
                <span
                  class="mt-0.5 flex items-center gap-1.5 text-xs text-muted-foreground"
                >
                  {#if card.merged}
                    <span>Merged into {trunk}</span>
                  {:else}
                    <span
                      class="font-mono whitespace-nowrap"
                      aria-label="{card.additions} lines added, {card.deletions} removed"
                    >
                      {#if card.additions > 0}<span class="text-success"
                          >+{card.additions}</span
                        >{/if}
                      {#if card.deletions > 0}<span class="text-destructive"
                          >−{card.deletions}</span
                        >{/if}
                      {#if card.additions === 0 && card.deletions === 0}±0{/if}
                    </span>
                    <span aria-hidden="true">·</span>
                    <span class="truncate">{commitsLabel(card.commits)}</span>
                  {/if}
                </span>
              </button>
              {#if pr}
                <button
                  type="button"
                  class="inline-flex shrink-0 items-center gap-1 rounded px-1 font-mono text-xs text-muted-foreground hover:text-foreground hover:underline disabled:no-underline"
                  disabled={!pr.url}
                  title="Pull request #{pr.number}, {pr.state}"
                  aria-label="Open pull request #{pr.number} on GitHub, {pr.state}"
                  onclick={() => pr.url && void openExternal(pr.url)}
                >
                  <GitPullRequest class="size-3" aria-hidden="true" />
                  #{pr.number}
                </button>
              {/if}
              {#if !card.current && !card.merged}
                <Button
                  size="icon-xs"
                  variant="ghost"
                  class="mr-1 shrink-0"
                  disabled={locked || held != null || switchingTo != null}
                  title={held
                    ? branchHoldLabel(held)
                    : locked
                      ? lockReason
                      : `Check out ${card.name} in this worktree`}
                  aria-label="Check out {card.name}"
                  onclick={() => void checkout(card.name)}
                >
                  {#if switching}
                    <LoaderCircle class="animate-spin" aria-hidden="true" />
                  {:else}
                    <GitBranch aria-hidden="true" />
                  {/if}
                </Button>
              {/if}
            </div>
          </li>
        {/each}

        <li class="flex gap-2" data-stack-trunk>
          <div class="flex w-4 shrink-0 justify-center" aria-hidden="true">
            <span
              class="mt-2 size-2.5 rotate-45 rounded-[2px] border border-muted-foreground/50 bg-muted"
            ></span>
          </div>
          <div
            class="flex items-center gap-2 py-1 text-xs text-muted-foreground"
          >
            <Layers class="size-3.5" aria-hidden="true" />
            <code class="font-mono">{trunk}</code>
            <span>trunk</span>
          </div>
        </li>
      </ol>
    </div>

    {#if stack?.status === 'unavailable'}
      <div
        class="mt-4 rounded-lg border border-border/70 bg-muted/20 p-3 text-xs"
        data-od-id="stack-unavailable"
      >
        <p>{unavailable.reason}</p>
        {#if unavailable.command}
          <pre
            class="mt-2 overflow-x-auto rounded-md border border-border/70 bg-background px-2 py-1.5 font-mono select-all">{unavailable.command}</pre>
        {/if}
        <p class="mt-2 text-muted-foreground">
          Stacks come from GitHub's <code class="font-mono">gh stack</code> extension,
          so they match what you see on github.com and in the terminal.
        </p>
      </div>
    {/if}
  </div>
</nav>
