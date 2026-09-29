<script lang="ts">
  import { onMount, tick } from 'svelte'
  import * as Dialog from '$lib/components/ui/dialog/index.js'
  import { Button } from '$lib/components/ui/button/index.js'
  import { Input } from '$lib/components/ui/input/index.js'
  import { Textarea } from '$lib/components/ui/textarea/index.js'
  import { Kbd, KbdGroup } from '$lib/components/ui/kbd/index.js'
  import { commands, fetchSnapshot } from '$lib/ipc'
  import { coreErrorText } from '$lib/feedback/core-error'
  import { showToast } from '$lib/feedback/show-toast'
  import { dismissOpenPopover } from '$lib/keyboard/global-shortcuts'
  import { draftWorkspaceName } from '$lib/new-workspace/draft'
  import { app, hydrateFromSnapshot, repos, shellDialogs } from '$lib/state'

  let open = $state(false)
  let title = $state('')
  let repoId = $state('')
  let prompt = $state('')
  /** The title is the user's while it holds text; clearing it hands it back to the draft. */
  let titleEdited = $state(false)
  let titleError = $state(false)
  let titleInput = $state<HTMLInputElement | null>(null)

  function defaultRepoId() {
    if (repos.items.some((repo) => repo.id === 'my-app')) return 'my-app'
    return repos.items[0]?.id ?? ''
  }

  async function prepareOpen() {
    dismissOpenPopover()
    window.dispatchEvent(new CustomEvent('cormux:close-palette'))
    titleEdited = false
    title = ''
    prompt = ''
    titleError = false
    repoId = defaultRepoId()
    open = true
    shellDialogs.newScratchOpen = true
    await tick()
    titleInput?.focus()
  }

  function closeDialog() {
    open = false
    shellDialogs.newScratchOpen = false
  }

  function onPromptInput() {
    if (titleEdited) return
    title = prompt.trim() ? draftWorkspaceName(prompt) : ''
    if (title.trim()) titleError = false
  }

  function onTitleInput() {
    titleEdited = title.length > 0
    if (title.trim()) titleError = false
  }

  async function submit() {
    const trimmed = title.trim()
    if (!trimmed) {
      titleError = true
      titleInput?.focus()
      return
    }
    // Nothing is provisioned, so the dialog closes before the core answers.
    const text = prompt.trim()
    closeDialog()
    const result = await commands.createScratch({
      title: trimmed,
      repoId,
      prompt: text || null,
    })
    if (result.status === 'error') {
      showToast({
        tone: 'bad',
        parts: [
          {
            type: 'text',
            value: coreErrorText(result.error, 'Could not start the scratch'),
          },
        ],
      })
      return
    }
    hydrateFromSnapshot(await fetchSnapshot())
    app.openScratch(result.data.scratchId, !text)
    showToast({
      tone: 'ok',
      parts: [{ type: 'text', value: `Started ${trimmed}` }],
    })
  }

  function onPromptKeydown(event: KeyboardEvent) {
    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
      event.preventDefault()
      void submit()
    }
  }

  onMount(() => {
    const onRequest = () => {
      void prepareOpen()
    }
    window.addEventListener('cormux:new-scratch', onRequest)
    return () => window.removeEventListener('cormux:new-scratch', onRequest)
  })
</script>

<Dialog.Root bind:open onOpenChange={(next) => !next && closeDialog()}>
  <Dialog.Content
    class="max-w-[480px] gap-0 p-0 sm:max-w-[480px]"
    aria-labelledby="session-dlg-title"
    data-od-id="session-dialog"
  >
    <form
      class="flex flex-col"
      novalidate
      onsubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
    >
      <Dialog.Header class="border-b px-4 py-3">
        <Dialog.Title id="session-dlg-title">New scratch</Dialog.Title>
      </Dialog.Header>

      <div class="flex flex-col gap-4 px-4 py-4">
        <div class="grid gap-1.5">
          <label class="text-sm font-medium" for="sess-title">Title</label>
          <Input
            bind:ref={titleInput}
            id="sess-title"
            maxlength={64}
            autocomplete="off"
            placeholder="Why the webhook signature fails"
            bind:value={title}
            oninput={onTitleInput}
            aria-invalid={titleError ? 'true' : undefined}
            aria-describedby="sess-title-hint sess-title-error"
          />
          <p
            id="sess-title-hint"
            class="text-xs text-muted-foreground"
            hidden={titleError}
          >
            Shown on the card and at the top of the scratch
          </p>
          <p
            id="sess-title-error"
            class="text-xs text-destructive"
            hidden={!titleError}
          >
            Give this scratch a title.
          </p>
        </div>

        <div class="grid gap-1.5">
          <label class="text-sm font-medium" for="sess-repo">Repository</label>
          <select
            id="sess-repo"
            class="border-input bg-input/30 h-8 w-full rounded-lg border px-2.5 text-sm text-foreground"
            bind:value={repoId}
            aria-describedby="sess-repo-hint"
          >
            {#each repos.items as repo (repo.id)}
              <option value={repo.id}>{repo.id}</option>
            {/each}
          </select>
          <p id="sess-repo-hint" class="text-xs text-muted-foreground">
            The agent reads this repo. No worktree or branch is created.
          </p>
        </div>

        <div class="grid gap-1.5">
          <label class="text-sm font-medium" for="sess-prompt">Prompt</label>
          <Textarea
            id="sess-prompt"
            rows={4}
            class="min-h-[112px] max-h-[280px] resize-y"
            placeholder="Ask something that doesn’t need its own branch"
            bind:value={prompt}
            oninput={onPromptInput}
            onkeydown={onPromptKeydown}
            aria-describedby="sess-prompt-hint"
          />
          <p id="sess-prompt-hint" class="text-xs text-muted-foreground">
            Optional. Leave it blank and write the first message in the scratch.
          </p>
        </div>
      </div>

      <Dialog.Footer class="items-center px-4 py-3 sm:justify-between">
        <p class="text-xs text-muted-foreground">Esc to cancel</p>
        <div class="flex gap-2">
          <Button type="button" variant="ghost" onclick={closeDialog}
            >Cancel</Button
          >
          <Button type="submit" disabled={!repoId}>
            Start scratch
            <KbdGroup class="ml-2 hidden sm:inline-flex">
              <Kbd>⌘</Kbd>
              <Kbd>↵</Kbd>
            </KbdGroup>
          </Button>
        </div>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
