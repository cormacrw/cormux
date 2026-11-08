<script lang="ts">
  import { untrack } from 'svelte'
  import { openWorkspace } from '$lib/nav/open'
  import { nav, navigate } from '$lib/core/nav.svelte'
  import { windowDrag } from '$lib/primitives/window-drag'
  import * as AlertDialog from '$lib/components/ui/alert-dialog'
  import { Badge } from '$lib/components/ui/badge'
  import DialogShortcut from '$lib/primitives/DialogShortcut.svelte'
  import { Button } from '$lib/components/ui/button'
  import { commands } from '$lib/ipc'
  import type { LocalBranchRow } from '$lib/ipc/bindings'
  import { coreErrorText } from '$lib/core/core-error'
  import { startsOver } from '$lib/core/owner-scoped'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { openExternal } from '$lib/feedback/open-external'
  import { onModEnter } from '$lib/keyboard/mod-enter'
  import { branchHoldLabel } from '$lib/repo/branch-hold'
  import { prs, repos, scratches, workspaces } from '$lib/state'
  import ExternalLink from '@lucide/svelte/icons/external-link'
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request'
  import Trash2 from '@lucide/svelte/icons/trash-2'

  let titleEl = $state<HTMLHeadingElement | null>(null)
  // Each repo's list starts over, so a Delete never names another repo's branch.
  let branches = $derived<LocalBranchRow[]>(startsOver(nav.repoId, []))
  let loading = $derived(startsOver(nav.repoId, true))
  let loadError = $derived<string | null>(startsOver(nav.repoId, null))
  let deleting = $derived<string | null>(startsOver(nav.repoId, null))
  /** A branch origin doesn't have as it is locally, awaiting a confirmed delete. */
  let unpushed = $derived<string | null>(startsOver(nav.repoId, null))
  // Held after close so the copy doesn't change while the dialog animates out.
  let unpushedShown = $state('')
  $effect(() => {
    if (unpushed) unpushedShown = unpushed
  })

  const repo = $derived(nav.repoId ? repos.getById(nav.repoId) : undefined)
  const defaultBranch = $derived(repo ? repos.defaultBranch(repo.id) : '')
  const git = $derived(repo ? repos.gitById[repo.id] : undefined)

  const repoWorkspaces = $derived(
    workspaces.items.filter((row) => row.repoId === repo?.id),
  )
  const repoScratches = $derived(
    scratches.items.filter((row) => row.repoId === repo?.id),
  )

  $effect(() =>
    nav.onArrival(() => {
      if (nav.view === 'repo') titleEl?.focus()
    }),
  )

  // Read again when the repo's workspaces move branches: the core says which it holds.
  const workspaceBranches = $derived(
    repoWorkspaces.map((row) => row.branch).join('\n'),
  )

  $effect(() => {
    const id = nav.repoId
    if (!id || nav.view !== 'repo') return
    void workspaceBranches
    let cancelled = false
    loading = untrack(() => branches.length === 0)
    loadError = null
    void commands.listRepoLocalBranches(id).then((result) => {
      if (cancelled) return
      loading = false
      if (result.status === 'error') {
        branches = []
        loadError = coreErrorText(result.error, 'Could not read branches')
        return
      }
      branches = result.data.branches
    })
    return () => {
      cancelled = true
    }
  })

  async function removeBranch(name: string, force = false) {
    const id = nav.repoId
    if (!id || deleting) return
    deleting = name
    const result = await commands.deleteLocalBranch(id, name, force)
    if (id !== nav.repoId) return
    deleting = null
    unpushed = null
    if (result.status === 'error') {
      toastCoreError(result.error)
      return
    }
    // Only a branch origin already has, at the same commit, goes without asking.
    if (result.data === 'unpushed') {
      unpushed = name
      return
    }
    branches = branches.filter((row) => row.name !== name)
  }

  const onUnpushedKeydown = onModEnter(() => {
    if (unpushed) void removeBranch(unpushed, true)
  })
</script>

<div
  class="view-repo flex min-h-0 flex-1 flex-col overflow-y-auto"
  data-od-id="repo-page"
>
  {#if repo}
    <div class="mx-auto flex w-full max-w-[760px] flex-col gap-10 p-6">
      <!-- The title block is the page's title bar: empty space in it drags the window. -->
      <header class="grid gap-3" data-tauri-drag-region use:windowDrag>
        <h1
          bind:this={titleEl}
          tabindex="-1"
          class="text-2xl font-semibold tracking-tight outline-none"
        >
          {repo.name}
        </h1>
        <p
          class="w-fit max-w-full truncate rounded-md bg-muted px-2 py-1 font-mono text-xs text-muted-foreground select-text"
        >
          {repo.path}
        </p>
        <div class="flex flex-wrap items-center gap-2">
          <Badge variant="outline" class="font-mono">{defaultBranch}</Badge>
          {#if git && (git.behind || git.ahead)}
            <span class="text-xs text-muted-foreground">
              {#if git.behind}
                <span class="text-amber-600 dark:text-amber-400"
                  >{git.behind} behind</span
                >
              {/if}
              {#if git.behind && git.ahead}
                <span aria-hidden="true">, </span>
              {/if}
              {#if git.ahead}
                <span class="text-sky-600 dark:text-sky-400"
                  >{git.ahead} ahead</span
                >
              {/if}
              of origin/{defaultBranch}
            </span>
          {/if}
          {#if repos.runCommand(repo.id)}
            <Badge variant="outline" class="max-w-full font-mono">
              <span class="truncate">Run {repos.runCommand(repo.id)}</span>
            </Badge>
          {/if}
        </div>
      </header>

      <section class="grid gap-3" aria-labelledby="repo-workspaces">
        <div class="flex items-center gap-2">
          <h2 id="repo-workspaces" class="text-sm font-medium tracking-tight">
            Workspaces
          </h2>
          <span
            class="rounded-full border border-border bg-muted/50 px-2 py-0.5 font-mono text-xs text-muted-foreground"
            >{repoWorkspaces.length}</span
          >
        </div>
        {#if repoWorkspaces.length === 0}
          <p class="text-sm text-muted-foreground">
            No workspaces for this repo yet.
          </p>
        {:else}
          <ul
            class="divide-y divide-border overflow-hidden rounded-xl border border-border bg-card"
          >
            {#each repoWorkspaces as row (row.id)}
              {@const status = row.activityText}
              <li>
                <button
                  type="button"
                  class="flex w-full items-start justify-between gap-4 px-4 py-3.5 text-left transition-colors hover:bg-muted/40"
                  onclick={() => openWorkspace(row.id)}
                >
                  <span class="min-w-0">
                    <span class="block truncate text-sm font-medium"
                      >{row.name}</span
                    >
                    <span
                      class="mt-1.5 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1"
                    >
                      <code
                        class="rounded bg-muted px-1.5 py-0.5 font-mono text-[11px]"
                        >{row.branch}</code
                      >
                      <span
                        class="truncate font-mono text-[11px] text-muted-foreground select-text"
                        title={row.worktreePath}>{row.worktreePath}</span
                      >
                    </span>
                  </span>
                  {#if status}
                    <Badge
                      variant="outline"
                      class="shrink-0 text-muted-foreground">{status}</Badge
                    >
                  {/if}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      <section class="grid gap-3" aria-labelledby="repo-branches">
        <div class="flex items-center gap-2">
          <h2 id="repo-branches" class="text-sm font-medium tracking-tight">
            Local branches
          </h2>
          <span
            class="rounded-full border border-border bg-muted/50 px-2 py-0.5 font-mono text-xs text-muted-foreground"
            >{branches.length}</span
          >
        </div>
        {#if loading}
          <p class="text-sm text-muted-foreground">Reading branches…</p>
        {:else if loadError}
          <p class="text-sm text-destructive">{loadError}</p>
        {:else if branches.length === 0}
          <p class="text-sm text-muted-foreground">No local branches.</p>
        {:else}
          <ul
            class="divide-y divide-border overflow-hidden rounded-xl border border-border bg-card"
          >
            {#each branches as branch (branch.name)}
              {@const reason = branch.hold && branchHoldLabel(branch.hold)}
              {@const pr = prs.forBranch(repo.id, branch.name)}
              <li
                class="flex items-start justify-between gap-4 px-4 py-3.5"
                data-od-id="repo-branch-{branch.name}"
              >
                <div class="min-w-0">
                  <div class="flex flex-wrap items-center gap-2">
                    <p class="truncate font-mono text-sm font-medium">
                      {branch.name}
                    </p>
                    {#if reason}
                      <Badge variant="outline" class="text-muted-foreground"
                        >{reason}</Badge
                      >
                    {/if}
                  </div>
                  {#if branch.subject || branch.committed}
                    <p class="mt-1 truncate text-xs text-muted-foreground">
                      {branch.subject}{branch.subject && branch.committed
                        ? ' · '
                        : ''}{branch.committed}
                    </p>
                  {/if}
                  {#if pr}
                    <button
                      type="button"
                      class="mt-2 inline-flex max-w-full items-center gap-1.5 rounded-md border border-border bg-background px-2 py-1 text-left text-xs transition-colors hover:bg-muted"
                      aria-label="Open pull request #{pr.num} on GitHub, {pr.title}"
                      onclick={() => void openExternal(pr.htmlUrl)}
                    >
                      <GitPullRequest
                        class="size-3.5 shrink-0 text-emerald-600 dark:text-emerald-400"
                        aria-hidden="true"
                      />
                      <span class="shrink-0 font-medium">PR #{pr.num}</span>
                      <span class="min-w-0 truncate text-muted-foreground"
                        >{pr.title}</span
                      >
                      <ExternalLink
                        class="size-3 shrink-0 text-muted-foreground"
                        aria-hidden="true"
                      />
                    </button>
                  {/if}
                </div>
                <Button
                  variant="destructive"
                  size="sm"
                  class="shrink-0"
                  disabled={reason !== null || deleting === branch.name}
                  aria-label={reason
                    ? `Delete ${branch.name}, ${reason}`
                    : `Delete ${branch.name}`}
                  onclick={() => void removeBranch(branch.name)}
                >
                  <Trash2 aria-hidden="true" />
                  Delete
                </Button>
              </li>
            {/each}
          </ul>
        {/if}
      </section>

      {#if repoScratches.length > 0}
        <section class="grid gap-3" aria-labelledby="repo-scratches">
          <div class="flex items-center gap-2">
            <h2 id="repo-scratches" class="text-sm font-medium tracking-tight">
              Scratches
            </h2>
            <span
              class="rounded-full border border-border bg-muted/50 px-2 py-0.5 font-mono text-xs text-muted-foreground"
              >{repoScratches.length}</span
            >
          </div>
          <ul
            class="divide-y divide-border overflow-hidden rounded-xl border border-border bg-card"
          >
            {#each repoScratches as scratch (scratch.id)}
              <li>
                <button
                  type="button"
                  class="w-full px-4 py-3.5 text-left text-sm font-medium transition-colors hover:bg-muted/40"
                  onclick={() =>
                    navigate({ view: 'scratch', scratchId: scratch.id })}
                >
                  {scratch.title}
                </button>
              </li>
            {/each}
          </ul>
        </section>
      {/if}
    </div>
  {/if}
</div>

<AlertDialog.Root
  open={unpushed != null}
  onOpenChange={(next) => {
    if (!next && !deleting) unpushed = null
  }}
>
  <AlertDialog.Content
    class="max-w-md sm:max-w-md"
    onkeydown={onUnpushedKeydown}
  >
    <AlertDialog.Header>
      <AlertDialog.Title
        >Delete a branch that isn't on origin?</AlertDialog.Title
      >
      <AlertDialog.Description>
        <code class="font-mono text-xs break-all">{unpushedShown}</code> isn't pushed
        to origin, or is out of sync with it. Commits that are only on this local
        branch will be lost.
      </AlertDialog.Description>
    </AlertDialog.Header>
    <AlertDialog.Footer>
      <AlertDialog.Cancel disabled={deleting != null}
        >Cancel <DialogShortcut keys="cancel" /></AlertDialog.Cancel
      >
      <AlertDialog.Action
        variant="destructive"
        disabled={deleting != null}
        onclick={(event) => {
          event.preventDefault()
          if (unpushed) void removeBranch(unpushed, true)
        }}
      >
        Delete anyway
        <DialogShortcut keys="submit" />
      </AlertDialog.Action>
    </AlertDialog.Footer>
  </AlertDialog.Content>
</AlertDialog.Root>
