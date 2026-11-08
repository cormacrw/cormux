<!--
  The first page: what a brand new thread looks like. There's no workspace behind it yet;
  sending the first message spawns one, in its own worktree, and opens it.
-->
<script lang="ts">
  import { openWorkspace } from '$lib/workspace/open'
  import { openSettings } from '$lib/settings/open'
  import { windowDrag } from '$lib/primitives/window-drag'
  import { tick, untrack } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import * as Select from '$lib/components/ui/select'
  import ThreadComposer from '$lib/components/workspace/thread/ThreadComposer.svelte'
  import ThreadHero from '$lib/components/workspace/thread/ThreadHero.svelte'
  import ThreadOptionPicker from '$lib/components/workspace/thread/ThreadOptionPicker.svelte'
  import type { ComposerAttachment } from '$lib/composer/attachments'
  import { commands } from '$lib/ipc'
  import { focusPromptAfterPicker } from '$lib/keyboard/focus-home'
  import { isHeaderShortcut } from '$lib/keyboard/header-shortcuts'
  import { shortcutLabel } from '$lib/keyboard/shortcuts'
  import {
    claimWorkspacePopover,
    releaseWorkspacePopover,
  } from '$lib/keyboard/popover-registry'
  import { resolveDefaultRepoId } from '$lib/new-workspace/settings-defaults'
  import { appearance, repos } from '$lib/state'
  import { nav } from '$lib/core/nav.svelte'
  import { settings } from '$lib/settings/store.svelte'
  import {
    enterConversation,
    firstPromptTransition,
  } from '$lib/conversation-motion'
  import { startThread } from '$lib/workspace/start-thread'
  import Folder from '@lucide/svelte/icons/folder'

  const LAST_REPO_KEY = 'cormux-last-repo'

  function lastRepo() {
    try {
      return globalThis.localStorage?.getItem(LAST_REPO_KEY) ?? ''
    } catch {
      return ''
    }
  }

  function rememberRepo(id: string) {
    try {
      globalThis.localStorage?.setItem(LAST_REPO_KEY, id)
    } catch {
      // Optional: the next thread just starts on the default repo.
    }
  }

  let repoId = $state('')
  let repoOpen = $state(false)
  let repoTrigger = $state<HTMLButtonElement | null>(null)
  let model = $state<string | null>(null)
  let effort = $state<string | null>(null)
  let focusComposer = $state<(() => void) | null>(null)
  let titleEl: HTMLHeadingElement | undefined = $state()

  const repo = $derived(repos.getById(repoId))

  // Repos arrive with the first snapshot, and the chosen one can be removed in Settings.
  $effect(() => {
    const items = repos.items
    untrack(() => {
      if (!items.some((row) => row.id === repoId)) {
        repoId = resolveDefaultRepoId(items, lastRepo() || settings.defaultRepo)
      }
    })
  })

  $effect(() => {
    if (nav.view !== 'newThread') return
    void nav.arrivals
    if (repos.items.length) focusComposer?.()
    else titleEl?.focus()
  })

  // Opening one picker from the keyboard closes the others.
  $effect(() => {
    if (!repoOpen) return
    const handle = {
      close: () => (repoOpen = false),
      restoreFocus: focusPromptAfterPicker,
    }
    claimWorkspacePopover(handle)
    return () => releaseWorkspacePopover(handle)
  })

  function onWindowKeydown(event: KeyboardEvent) {
    if (!isHeaderShortcut(event, 'pickRepo') || !repos.items.length) return
    event.preventDefault()
    // The list takes its arrow keys and ↵ through the focused trigger.
    repoTrigger?.focus()
    repoOpen = !repoOpen
  }

  async function submit(
    text: string,
    stay: boolean,
    attachments: ComposerAttachment[],
  ) {
    const id = repoId
    const workspaceId = await startThread({
      repoId: id,
      prompt: text,
      model,
      effort,
      attachments,
    })
    if (!workspaceId) return false
    rememberRepo(id)
    if (stay) return true
    // The greeting falls away and the conversation rises in, as on a first prompt in a thread.
    firstPromptTransition(
      () => openWorkspace(workspaceId),
      () => appearance.glideWordmark(),
    )
    void tick().then(enterConversation)
    return true
  }

  const listSkills = $derived.by(() => {
    const id = repoId
    return async () => {
      const result = await commands.listRepoSkills(id)
      return result.status === 'ok' ? result.data : []
    }
  })
</script>

<svelte:window onkeydown={onWindowKeydown} />

<section
  class="flex min-h-0 flex-1 flex-col"
  aria-labelledby="new-thread-title"
  data-od-id="new-thread"
>
  <header
    class="flex items-center px-4 py-3"
    data-tauri-drag-region
    use:windowDrag
  >
    <h1
      bind:this={titleEl}
      id="new-thread-title"
      tabindex="-1"
      class="text-[15px] font-medium tracking-tight outline-none"
    >
      New thread
    </h1>
  </header>

  <div class="relative flex min-h-0 flex-1 flex-col">
    <div class="@container min-h-0 flex-1 overflow-y-auto">
      <div
        class="mx-auto flex min-h-full w-full max-w-[760px] flex-col overflow-clip px-4 pt-3 select-text"
      >
        {#if repos.items.length}
          <ThreadHero greeting="new-thread:new" />
        {:else}
          <div
            class="mt-auto flex flex-col items-start gap-3 pb-10"
            data-od-id="new-thread-no-repos"
          >
            <h2 class="text-4xl font-semibold tracking-tight">
              Add a repo to get started
            </h2>
            <p class="text-sm text-muted-foreground">
              Every thread works in its own worktree of a repo.
            </p>
            <Button
              variant="secondary"
              onclick={() => {
                openSettings({ section: 'repos' })
              }}
            >
              Add a repo
            </Button>
          </div>
        {/if}
      </div>
    </div>

    <ThreadComposer
      draftKey="new-thread"
      bind:focusComposer
      inputLabel="Start a new thread"
      placeholder={repo
        ? `What should we do in ${repo.name}?`
        : 'Add a repo to start a thread'}
      sendLabel="Start thread"
      disabled={!repo}
      onSubmit={submit}
      {listSkills}
    >
      {#snippet pickers()}
        {#if repos.items.length}
          <label class="sr-only" for="new-thread-repo">Repository</label>
          <Select.Root
            type="single"
            bind:open={repoOpen}
            onOpenChangeComplete={(open) => {
              // Back to the prompt rather than the picker's button.
              if (!open) focusPromptAfterPicker()
            }}
            value={repoId}
            onValueChange={(value) => {
              if (value) repoId = value
            }}
          >
            <Select.Trigger
              bind:ref={repoTrigger}
              id="new-thread-repo"
              size="sm"
              class="glass-well h-7 max-w-[168px] min-w-0 shrink gap-1.5 rounded-[9px] border-transparent text-xs"
              title="The worktree is created from this repo ({shortcutLabel(
                'pickRepo',
              )})"
              data-od-id="new-thread-repo"
            >
              <Folder class="size-3.5 shrink-0 opacity-70" aria-hidden="true" />
              <span class="truncate">{repo?.name ?? 'Pick a repository'}</span>
            </Select.Trigger>
            <Select.Content>
              {#each repos.items as row (row.id)}
                <Select.Item value={row.id} label={row.name}>
                  {row.name}
                </Select.Item>
              {/each}
            </Select.Content>
          </Select.Root>
          <ThreadOptionPicker
            kind="model"
            threadId={null}
            bind:draftValue={model}
          />
          <ThreadOptionPicker
            kind="effort"
            threadId={null}
            bind:draftValue={effort}
          />
        {/if}
      {/snippet}
    </ThreadComposer>
  </div>
</section>
