<!--
  The first page: what a brand new thread looks like. There's no workspace behind it yet;
  sending the first message spawns one, in its own worktree, and opens it.
-->
<script lang="ts">
  import { openWorkspace } from '$lib/nav/open'
  import { openSettings } from '$lib/settings/open'
  import { windowDrag } from '$lib/primitives/window-drag'
  import { untrack } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import * as Select from '$lib/components/ui/select'
  import ThreadComposer, {
    type ComposerTarget,
  } from '$lib/components/workspace/thread/ThreadComposer.svelte'
  import ThreadHero, {
    HERO_COLUMN,
  } from '$lib/components/workspace/thread/ThreadHero.svelte'
  import ThreadOptionPicker, {
    draftOptions,
  } from '$lib/components/workspace/thread/ThreadOptionPicker.svelte'
  import type { ComposerAttachment } from '$lib/composer/attachments'
  import { commands } from '$lib/ipc'
  import { focusPrompt, focusPromptAfterPicker } from '$lib/keyboard/focus-home'
  import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
  import { shortcutLabel } from '$lib/keyboard/shortcuts'
  import { keyboardLayers } from '$lib/keyboard/layers.svelte'
  import { resolveDefaultRepoId } from '$lib/settings/default-repo'
  import { repos } from '$lib/state'
  import { wordmark } from '$lib/background/wordmark.svelte'
  import { nav } from '$lib/core/nav.svelte'
  import { settings } from '$lib/settings/store.svelte'
  import {
    enterConversation,
    firstPromptTransition,
  } from '$lib/thread/conversation-motion'
  import { startThread } from '$lib/workspace/start-thread'
  import Folder from '@lucide/svelte/icons/folder'

  let repoId = $state('')
  let repoOpen = $state(false)
  let repoTrigger = $state<HTMLButtonElement | null>(null)
  let model = $state<string | null>(null)
  let effort = $state<string | null>(null)
  let titleEl: HTMLHeadingElement | undefined = $state()

  const repo = $derived(repos.getById(repoId))

  // Repos arrive with the first snapshot, and the chosen one can be removed in Settings.
  $effect(() => {
    const items = repos.items
    untrack(() => {
      if (!items.some((row) => row.id === repoId)) {
        repoId = resolveDefaultRepoId(items, settings.values.defaultRepo)
      }
    })
  })

  $effect(() =>
    nav.onArrival(() => {
      if (nav.view !== 'newThread') return
      if (repos.items.length) focusPrompt()
      else titleEl?.focus()
    }),
  )

  $effect(() => {
    if (!repoOpen) return
    const handle = {
      close: () => (repoOpen = false),
      restoreFocus: focusPromptAfterPicker,
    }
    keyboardLayers.claimPopover(handle)
    return () => keyboardLayers.releasePopover(handle)
  })

  function onWindowKeydown(event: KeyboardEvent) {
    if (!shellDialogs.allows(event, 'pickRepo') || !repos.items.length) return
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
    if (stay) return true
    // The greeting falls away and the conversation rises in, as on a first prompt in a thread.
    firstPromptTransition(
      () => openWorkspace(workspaceId),
      () => wordmark.glide(),
      enterConversation,
    )
    return true
  }

  // A new thread's skills come from the repo it will start in.
  const listSkills = $derived.by(() => {
    const id = repoId
    return () => commands.listRepoSkills(id)
  })
  const composer: ComposerTarget = $derived({
    draftKey: 'new-thread',
    inputLabel: 'Start a new thread',
    sendLabel: 'Start thread',
    placeholder: repo
      ? `What should we do in ${repo.name}?`
      : 'Add a repo to start a thread',
    disabled: !repo,
    send: submit,
    listSkills,
    promptHistory: [],
    stop: null,
    pickers,
  })
  const modelSource = draftOptions('model', {
    get current() {
      return model
    },
    set: (next) => (model = next),
  })
  const effortSource = draftOptions('effort', {
    get current() {
      return effort
    },
    set: (next) => (effort = next),
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
      <div class={HERO_COLUMN}>
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

    <ThreadComposer target={composer} />
  </div>
</section>

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
    <ThreadOptionPicker kind="model" source={modelSource} />
    <ThreadOptionPicker kind="effort" source={effortSource} />
  {/if}
{/snippet}
