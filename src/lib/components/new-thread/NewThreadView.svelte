<!--
  The first page: what a brand new thread looks like. There's no workspace behind it yet;
  sending the first message spawns one, in its own worktree, and opens it.
-->
<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { tick, untrack } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import * as Select from '$lib/components/ui/select'
  import ThreadComposer from '$lib/components/workspace/thread/ThreadComposer.svelte'
  import ThreadHero from '$lib/components/workspace/thread/ThreadHero.svelte'
  import ThreadModelPicker from '$lib/components/workspace/thread/ThreadModelPicker.svelte'
  import { commands } from '$lib/ipc'
  import { resolveDefaultRepoId } from '$lib/new-workspace/settings-defaults'
  import { app, repos, settings } from '$lib/state'
  import type { Thread } from '$lib/state/threads.svelte'
  import { NO_USAGE } from '$lib/usage/usage'
  import {
    enterConversation,
    firstPromptTransition,
  } from '$lib/thread/conversation-motion'
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

  // A stand-in the composer and greeting can draw until there's a real thread.
  const draftThread = $derived<Thread>({
    id: 'new-thread',
    workspaceId: '',
    role: 'Lead',
    engine: settings.defaultEngine,
    status: 'idle',
    paused: false,
    activity: '',
    pendingApprovals: 0,
    usage: NO_USAGE,
    retired: false,
  })

  let repoId = $state('')
  let model = $state<string | null>(null)
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
    if (app.focusTarget !== 'newThread') return
    void app.focusGeneration
    if (repos.items.length) focusComposer?.()
    else titleEl?.focus()
  })

  async function submit(text: string, stay: boolean) {
    const id = repoId
    const workspaceId = await startThread({
      repoId: id,
      prompt: text,
      model,
    })
    if (!workspaceId) return false
    rememberRepo(id)
    if (stay) return true
    // The greeting falls away and the conversation rises in, as on a first prompt in a thread.
    firstPromptTransition(() => app.openWorkspace(workspaceId))
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

  function startHeaderDrag(event: MouseEvent) {
    if (event.button !== 0) return
    const target = event.target
    if (!(target instanceof Element)) return
    if (target.closest('button, a, input, textarea')) return
    void getCurrentWindow().startDragging()
  }
</script>

<section
  class="flex min-h-0 flex-1 flex-col"
  aria-labelledby="new-thread-title"
  data-od-id="new-thread"
>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header
    class="flex items-center px-4 py-3"
    data-tauri-drag-region
    onmousedown={startHeaderDrag}
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
          <ThreadHero
            thread={draftThread}
            workspace={null}
            conversationId="new"
            earlier={[]}
            nowMs={0}
          />
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
                settings.focusSection = 'repos'
                app.openSettings()
              }}
            >
              Add a repo
            </Button>
          </div>
        {/if}
      </div>
    </div>

    <ThreadComposer
      thread={draftThread}
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
            value={repoId}
            onValueChange={(value) => {
              if (value) repoId = value
            }}
          >
            <Select.Trigger
              id="new-thread-repo"
              size="sm"
              class="glass-well h-7 max-w-[168px] min-w-0 shrink gap-1.5 rounded-[9px] border-transparent text-xs"
              title="The worktree is created from this repo"
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
          <ThreadModelPicker
            thread={draftThread}
            draft
            bind:draftModel={model}
          />
        {/if}
      {/snippet}
    </ThreadComposer>
  </div>
</section>
