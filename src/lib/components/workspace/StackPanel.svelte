<script lang="ts">
  import { tick } from 'svelte'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import GitBranch from '@lucide/svelte/icons/git-branch'
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request'
  import Layers from '@lucide/svelte/icons/layers'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import Plus from '@lucide/svelte/icons/plus'
  import RefreshCw from '@lucide/svelte/icons/refresh-cw'
  import Upload from '@lucide/svelte/icons/upload'
  import { Badge } from '$lib/components/ui/badge'
  import { Button } from '$lib/components/ui/button'
  import { prs, stacks, threads, workspaceRecords } from '$lib/state'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import {
    branchErrorMessage,
    validateBranchName,
  } from '$lib/new-workspace/validation'
  import {
    canAddToStack,
    pullRequestForBranch,
    commitsLabel,
    prStateLabel,
    stackCardsTopFirst,
    stackSubtitle,
  } from '$lib/stack/stack'
  import { branchPickerLocked } from '$lib/workspace/running-agents'
  import {
    addStackBranch,
    pushStack,
    switchWorkspaceBranch,
    syncStack,
  } from '$lib/workspace/wire-git-workspace'
  import { cn } from '$lib/utils'

  let {
    workspace,
    repoId,
    provisioning,
  }: { workspace: Workspace; repoId: string; provisioning: boolean } = $props()

  const stack = $derived(stacks.get(workspace.id))
  const cards = $derived(stackCardsTopFirst(stack))
  const trunk = $derived(stack?.trunk ?? 'main')
  const stacked = $derived(stack?.status === 'stacked')
  const locked = $derived(
    provisioning || branchPickerLocked(threads.forWorkspace(workspace.id)),
  )
  const lockReason = 'Pause or stop the running agents to change branches.'
  const addable = $derived(canAddToStack(stack))
  // Backend messages end with " Run: <command>" when there's something to install.
  const unavailable = $derived.by(() => {
    const [reason, command] = (stack?.message ?? '').split(' Run: ')
    return { reason, command }
  })
  const topBranch = $derived(cards[0]?.name)
  const addTitle = $derived(
    locked
      ? lockReason
      : !addable && topBranch
        ? `New branches go on top. Check out ${topBranch} first.`
        : `Create a branch on top of ${workspace.branch}`,
  )
  const otherWorkspaceBranches = $derived(
    workspaceRecords.records
      .filter(
        (record) => record.repoId === repoId && record.id !== workspace.id,
      )
      .map((record) => record.branch),
  )

  let busy = $state<'push' | 'sync' | null>(null)
  let switchingTo = $state<string | null>(null)
  let adding = $state(false)
  let creating = $state(false)
  let newBranch = $state('')
  let addError = $state<string | null>(null)
  let addInput = $state<HTMLInputElement | null>(null)

  /** A PR opened by hand shows up here before gh-stack learns of it on the next Sync. */
  function syncedPr(branch: string) {
    const pr = pullRequestForBranch(prs.items, repoId, branch)
    return pr ? { number: pr.num, url: pr.htmlUrl, state: 'OPEN' } : null
  }

  async function checkout(name: string) {
    if (locked || switchingTo) return
    switchingTo = name
    await switchWorkspaceBranch(workspace.id, name)
    switchingTo = null
  }

  async function runRemote(action: 'push' | 'sync') {
    if (busy) return
    busy = action
    const ok = await (action === 'push' ? pushStack : syncStack)(workspace.id)
    busy = null
    // A branch switch reloads the stack on its own; rebases and pushes don't.
    if (ok) await stacks.load(workspace.id)
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
    const name = newBranch.trim()
    const code = validateBranchName(name, {
      existingBranches: [trunk, ...cards.map((card) => card.name)],
      workspaceBranches: otherWorkspaceBranches,
    })
    if (code) {
      addError = branchErrorMessage(name, code)
      return
    }
    creating = true
    const ok = await addStackBranch(workspace.id, name)
    creating = false
    if (ok) closeAdd()
  }
</script>

<section
  id="stack-panel"
  role="tabpanel"
  aria-labelledby="thread-tab-stack"
  data-od-id="stack"
  class="flex min-h-0 flex-1 flex-col"
>
  <div class="min-h-0 flex-1 overflow-y-auto">
    <div class="mx-auto w-full max-w-2xl px-8 pt-8 pb-12">
      <header class="flex items-start justify-between gap-4">
        <div class="min-w-0 space-y-1">
          <h2 class="text-xl font-semibold tracking-tight">Stack</h2>
          <p class="text-sm text-muted-foreground">
            {stack ? stackSubtitle(stack) : 'Reading the stack…'}
          </p>
        </div>
        {#if stack && stack.status !== 'unavailable'}
          <div class="flex shrink-0 items-center gap-1.5">
            {#if stacked}
              <Button
                size="sm"
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
            {/if}
            {#if !adding}
              <Button
                size="sm"
                variant={stacked ? 'ghost' : 'default'}
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
                size="sm"
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

      {#if stack?.status === 'unavailable'}
        <div
          class="mt-6 rounded-lg border border-border/70 bg-muted/20 p-4 text-sm"
          data-od-id="stack-unavailable"
        >
          <p>{unavailable.reason}</p>
          {#if unavailable.command}
            <pre
              class="mt-2 rounded-md border border-border/70 bg-background px-3 py-2 font-mono text-xs select-all">{unavailable.command}</pre>
          {/if}
          <p class="mt-2 text-muted-foreground">
            Stacks come from GitHub's <code class="font-mono">gh stack</code> extension,
            so they match what you see on github.com and in the terminal.
          </p>
        </div>
      {:else if stack}
        <div class="relative mt-6">
          <span
            class="absolute top-5 bottom-3 left-[7.5px] w-px bg-border"
            aria-hidden="true"
          ></span>
          <ol class="relative" aria-label="Branches in this stack, top first">
            {#if adding}
              <li class="relative flex gap-3 pb-3">
                <div
                  class="flex w-4 shrink-0 justify-center"
                  aria-hidden="true"
                >
                  <span
                    class="mt-3.5 size-2.5 rounded-full border-2 border-dashed border-muted-foreground/60 bg-background"
                  ></span>
                </div>
                <form
                  class="flex-1 rounded-lg border border-dashed border-border bg-muted/20 p-3"
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
                  <div class="mt-2 flex gap-2">
                    <input
                      bind:this={addInput}
                      id="stack-new-branch"
                      bind:value={newBranch}
                      placeholder="feat/next-step"
                      autocomplete="off"
                      spellcheck="false"
                      aria-invalid={addError != null}
                      aria-describedby="stack-add-hint"
                      class="min-w-0 flex-1 rounded-md border border-input bg-background px-2 py-1 font-mono text-xs outline-none focus-visible:border-ring focus-visible:ring-2 focus-visible:ring-ring/40"
                      oninput={() => (addError = null)}
                      onkeydown={(event) => {
                        if (event.key === 'Escape') {
                          event.stopPropagation()
                          closeAdd()
                        }
                      }}
                    />
                    <Button
                      type="submit"
                      size="sm"
                      variant="secondary"
                      disabled={creating || !newBranch.trim()}
                    >
                      {#if creating}
                        <LoaderCircle class="animate-spin" aria-hidden="true" />
                      {/if}
                      Create
                    </Button>
                    <Button
                      type="button"
                      size="sm"
                      variant="ghost"
                      onclick={closeAdd}
                    >
                      Cancel
                    </Button>
                  </div>
                  <p
                    id="stack-add-hint"
                    class={cn(
                      'mt-2 text-xs',
                      addError ? 'text-destructive' : 'text-muted-foreground',
                    )}
                  >
                    {addError ?? 'Uncommitted changes move to the new branch.'}
                  </p>
                </form>
              </li>
            {/if}

            {#if !stacked && workspace.branch !== trunk}
              <li
                class="relative flex gap-3 pb-3"
                data-stack-branch={workspace.branch}
              >
                <div
                  class="flex w-4 shrink-0 justify-center"
                  aria-hidden="true"
                >
                  <span
                    class="mt-3.5 size-2.5 shrink-0 rounded-full bg-primary ring-4 ring-primary/20"
                  ></span>
                </div>
                <div
                  class="flex min-w-0 flex-1 items-center gap-2 rounded-lg border border-dashed border-border/70 px-3 py-2.5"
                >
                  <span class="truncate font-mono text-sm"
                    >{workspace.branch}</span
                  >
                  <Badge variant="secondary" class="shrink-0 px-1.5 text-[10px]"
                    >not stacked</Badge
                  >
                </div>
              </li>
            {/if}

            {#each cards as card (card.name)}
              {@const switching = switchingTo === card.name}
              {@const pr = card.pr ?? syncedPr(card.name)}
              {@const elsewhere = otherWorkspaceBranches.includes(card.name)}
              <li
                class="relative flex gap-3 pb-3"
                data-stack-branch={card.name}
              >
                <div
                  class="flex w-4 shrink-0 justify-center"
                  aria-hidden="true"
                >
                  <span
                    class={cn(
                      'mt-3.5 size-2.5 shrink-0 rounded-full',
                      card.current
                        ? 'bg-primary ring-4 ring-primary/20'
                        : card.merged
                          ? 'bg-muted-foreground/50'
                          : 'border-2 border-muted-foreground/50 bg-background',
                    )}
                  ></span>
                </div>
                <div
                  class={cn(
                    'flex min-w-0 flex-1 items-center gap-3 rounded-lg border px-3 py-2.5 transition-colors',
                    card.current
                      ? 'border-primary/40 bg-primary/5'
                      : 'border-border/70 hover:bg-muted/30',
                    card.merged && 'opacity-60',
                  )}
                  aria-current={card.current ? 'true' : undefined}
                >
                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2">
                      <span class="truncate font-mono text-sm" title={card.name}
                        >{card.name}</span
                      >
                      {#if card.current}
                        <Badge
                          variant="secondary"
                          class="shrink-0 px-1.5 text-[10px]">current</Badge
                        >
                      {/if}
                      {#if card.needsRebase}
                        <Badge
                          variant="outline"
                          class="shrink-0 border-warning/40 bg-warning/10 px-1.5 text-[10px] text-warning"
                          title="{card.parent} moved on since this branch was based on it. Sync to rebase."
                          >needs rebase</Badge
                        >
                      {/if}
                    </div>
                    <div
                      class="mt-0.5 flex items-center gap-2 text-xs text-muted-foreground"
                    >
                      {#if card.merged}
                        <span>Merged into {trunk}</span>
                      {:else}
                        <span
                          class="font-mono"
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
                        <span>{commitsLabel(card.commits)}</span>
                      {/if}
                    </div>
                  </div>
                  {#if pr}
                    <button
                      type="button"
                      class="inline-flex shrink-0 items-center gap-1 rounded px-1 font-mono text-xs text-muted-foreground hover:text-foreground hover:underline disabled:no-underline"
                      disabled={!pr.url}
                      title="Pull request #{pr.number}, {prStateLabel(
                        pr.state,
                      )}"
                      aria-label="Open pull request #{pr.number} on GitHub, {prStateLabel(
                        pr.state,
                      )}"
                      onclick={() => pr.url && void openUrl(pr.url)}
                    >
                      <GitPullRequest class="size-3" aria-hidden="true" />
                      #{pr.number}
                    </button>
                  {/if}
                  {#if card.current}
                    <span class="shrink-0 text-xs text-muted-foreground"
                      >Checked out</span
                    >
                  {:else if !card.merged}
                    <Button
                      size="xs"
                      variant="outline"
                      class="shrink-0"
                      disabled={locked || elsewhere || switchingTo != null}
                      title={elsewhere
                        ? 'Checked out in another workspace’s worktree'
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
                      Check out
                    </Button>
                  {/if}
                </div>
              </li>
            {/each}

            <li class="flex gap-3" data-stack-trunk>
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
      {/if}
    </div>
  </div>
</section>
