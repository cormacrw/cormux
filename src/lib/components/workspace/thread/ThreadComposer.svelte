<script lang="ts" module>
  import type { Snippet } from 'svelte'
  import type { ComposerAttachment } from '$lib/composer/attachments'
  import type { CoreError } from '$lib/ipc'
  import type { Result, Skill } from '$lib/ipc/bindings'
  import type { Thread } from '$lib/state/threads.svelte'

  /** Who a message goes to: a thread, or a new one that the first message starts. */
  export type ComposerTarget =
    | { thread: Thread }
    | {
        /** Where the draft is kept until the thread exists. */
        draftKey: string
        /** The repo it will start in, for the placeholder; null with none to pick. */
        repoName: string | null
        /**
         * Starts the thread with the message; resolves false to put the draft back. `stay` is
         * ⌘↵: send it off without leaving.
         */
        start: (
          text: string,
          stay: boolean,
          attachments: ComposerAttachment[],
        ) => Promise<boolean>
        /** The `/` picker's skills, read from the repo it will start in. */
        listSkills: () => Promise<Result<Skill[], CoreError>>
        /** Its own repo, model and effort pickers, at the start of the footer. */
        pickers: Snippet
      }
</script>

<script lang="ts">
  import {
    sendToThread,
    setThreadPaused,
    stopThread,
    switchConversation,
  } from '$lib/thread/actions'
  import { onMount, tick } from 'svelte'
  import { SvelteSet } from 'svelte/reactivity'
  import { startsOver } from '$lib/core/owner-scoped'
  import {
    composerMayTakeFocus,
    isEditable,
    registerPrompt,
  } from '$lib/keyboard/focus-home'
  import { keyboardLayers } from '$lib/keyboard/layers.svelte'
  import { nav } from '$lib/core/nav.svelte'
  import { getCurrentWebview } from '@tauri-apps/api/webview'
  import { open as openFilePicker } from '@tauri-apps/plugin-dialog'
  import { Button } from '$lib/components/ui/button'
  import {
    attachmentFromFile,
    attachmentFromPath,
    isImageName,
  } from '$lib/composer/attachments'
  import { Textarea } from '$lib/components/ui/textarea'
  import {
    composerPauseLabel,
    composerShowsPauseControl,
  } from '$lib/composer/can-pause'
  import { stepPromptHistory } from '$lib/composer/prompt-history'
  import { commands } from '$lib/ipc'
  import {
    pickerAt,
    replaceWord,
    type TriggerWord,
  } from '$lib/composer/triggers'
  import { expandMessage } from '$lib/composer/expand-message'
  import { bangPreview } from '$lib/bangs/bangs'
  import Zap from '@lucide/svelte/icons/zap'
  import { clickup } from '$lib/clickup/store.svelte'
  import { repos } from '$lib/state/repos.svelte'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { composerDrafts } from '$lib/composer/drafts-store.svelte'
  import { settings } from '$lib/settings/store.svelte'
  import { contextPercent, usageDetail } from '$lib/usage/usage'
  import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
  import { shortcutLabel } from '$lib/keyboard/shortcuts'
  import { threadTimeline } from '$lib/thread/timeline-store.svelte'
  import { workspaceUi } from '$lib/state/workspace-ui.svelte'
  import ThreadOptionPicker from './ThreadOptionPicker.svelte'
  import ArrowUp from '@lucide/svelte/icons/arrow-up'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import FileText from '@lucide/svelte/icons/file-text'
  import Folder from '@lucide/svelte/icons/folder'
  import Kanban from '@lucide/svelte/icons/square-kanban'
  import Paperclip from '@lucide/svelte/icons/paperclip'
  import X from '@lucide/svelte/icons/x'
  import Pause from '@lucide/svelte/icons/pause'
  import Play from '@lucide/svelte/icons/play'
  import Sparkles from '@lucide/svelte/icons/sparkles'
  import Square from '@lucide/svelte/icons/square'

  let {
    target,
    onSent,
    status,
  }: {
    target: ComposerTarget
    onSent?: () => void
    /** What the agent is doing, floated just above the prompt bar. */
    status?: Snippet
  } = $props()

  const thread = $derived('thread' in target ? target.thread : undefined)
  const newThread = $derived('thread' in target ? undefined : target)
  const disabled = $derived(newThread?.repoName === null)
  const placeholder = $derived(
    thread
      ? `Message ${thread.role}…`
      : newThread?.repoName
        ? `What should we do in ${newThread.repoName}?`
        : 'Add a repo to start a thread',
  )

  let inputEl = $state<HTMLTextAreaElement | null>(null)
  let composing = $state(false)

  const draftId = $derived(thread?.id ?? newThread?.draftKey ?? '')
  /** Which past prompt ↑/↓ recalled into the composer; null when not cycling. */
  let historyIndex = $derived<number | null>(startsOver(draftId, null))
  const draft = $derived(composerDrafts.textFor(draftId))
  const threadStatus = $derived(thread?.status ?? 'idle')
  const showPause = $derived(composerShowsPauseControl(threadStatus))
  const pauseLabel = $derived(composerPauseLabel(threadStatus))
  const isPaused = $derived(threadStatus === 'paused')
  // Only how full the context is; tokens and cost are in the thread tab's tooltip.
  const contextFill = $derived(thread ? contextPercent(thread.usage) : null)
  const usageTooltip = $derived(thread ? usageDetail(thread.usage) : undefined)
  const view = $derived(thread ? threadTimeline.threadView(thread.id) : null)
  const attachments = $derived(composerDrafts.attachmentsFor(draftId))
  // Drafts between Send and the core taking the message, which can mean starting the agent first.
  const sendingDrafts = new SvelteSet<string>()
  const sending = $derived(sendingDrafts.has(draftId))
  const canSend = $derived(
    !disabled &&
      !sending &&
      (draft.trim().length > 0 || attachments.length > 0),
  )
  let draggingFiles = $state(false)

  // Typing `/` opens the skill picker, `@` the repos set up in Cormux (the folder goes in on
  // send), `#` your sprint tasks, and `!` your bangs (picking one puts its prompt in).
  // A new thread's skills come from the repo it will start in.
  const skillsOwner = $derived(newThread?.listSkills ?? draftId)
  let skills = $derived<Skill[] | null>(startsOver(skillsOwner, null))
  let skillsAsked = $derived(startsOver(skillsOwner, false))
  let pickerIndex = $derived(startsOver(draftId, 0))
  /** Esc hides the picker until the draft changes. */
  let pickerDismissed = $derived(startsOver(draftId, false))
  /** Where the caret is in the draft; the picker follows the trigger word it's in. */
  let caret = $derived(startsOver(draftId, 0))
  const sprintTasks = $derived(clickup.mine?.tasks ?? [])
  const picker = $derived(
    pickerAt(draft, caret, {
      skills,
      repos: repos.items,
      tasks: sprintTasks,
      bangs: settings.values.bangs,
    }),
  )
  const pickerOptions = $derived.by(() => {
    if (!picker) return []
    const base = { kind: picker.kind, choose: chooseFrom(picker.word) }
    switch (picker.kind) {
      case 'skill':
        return picker.options.map((option) => ({
          ...base,
          ...option,
          detail: option.item.description,
          badge:
            option.item.source === 'project'
              ? 'repo'
              : option.item.source === 'plugin'
                ? 'plugin'
                : null,
        }))
      case 'bang':
        return picker.options.map((option) => ({
          ...base,
          ...option,
          detail: bangPreview(option.item.prompt, 80),
          badge: null,
        }))
      case 'task':
        return picker.options.map((option) => ({
          ...base,
          ...option,
          detail: option.item.name,
          badge: option.item.status,
        }))
      case 'repo':
        return picker.options.map((option) => ({
          ...base,
          ...option,
          detail: option.item.path,
          badge: null,
        }))
    }
  })
  const pickerOpen = $derived(!pickerDismissed && pickerOptions.length > 0)

  $effect(() => {
    if (picker?.kind !== 'skill' || skillsAsked) return
    const owner = skillsOwner
    skillsAsked = true
    const read = newThread
      ? newThread.listSkills()
      : commands.listThreadSkills(draftId)
    void read.then((result) => {
      if (result.status === 'error')
        toastCoreError(result.error, 'Could not read the skills')
      if (skillsOwner === owner)
        skills = result.status === 'ok' ? result.data : []
    })
  })

  function chooseFrom(word: TriggerWord) {
    return (option: { insert: string; as: 'word' | 'text' }) =>
      applyPick(replaceWord(draft, word, option.insert, option.as))
  }

  function applyPick(next: { text: string; caret: number }) {
    composerDrafts.setFor(draftId, next.text)
    caret = next.caret
    pickerIndex = 0
    void tick().then(() => {
      fitHeight()
      inputEl?.focus()
      inputEl?.setSelectionRange(next.caret, next.caret)
    })
  }

  function syncCaret() {
    if (inputEl) caret = inputEl.selectionStart
  }

  function onPickerKeydown(event: KeyboardEvent): boolean {
    const count = pickerOptions.length
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      const step = event.key === 'ArrowDown' ? 1 : -1
      pickerIndex = (pickerIndex + step + count) % count
    } else if (
      (event.key === 'Enter' && !event.shiftKey) ||
      event.key === 'Tab'
    ) {
      const option = pickerOptions[pickerIndex]
      option?.choose(option)
    } else if (event.key === 'Escape') {
      pickerDismissed = true
    } else {
      return false
    }
    event.preventDefault()
    return true
  }

  // Fits the box to text put in the draft from outside too (a picked task).
  $effect(() => {
    void draft
    fitHeight()
  })

  // Esc stops the agent once nothing in front of it (a popover, a slide-out) wanted the key.
  $effect(() =>
    keyboardLayers.onEscape('agent', () => {
      if (threadStatus !== 'running') return false
      stopTurn()
      return true
    }),
  )

  $effect(() => registerPrompt(() => inputEl))

  /**
   * Puts focus back in the prompt, unless the user is busy somewhere else. It waits for Svelte
   * to draw what the trigger changed (a button swapped out, a menu opened), so the check sees
   * the page as it now is.
   */
  function takeFocusSoon() {
    void tick().then(() => {
      if (inputEl && composerMayTakeFocus(inputEl))
        inputEl.focus({ preventScroll: true })
    })
  }

  // Opening a conversation, switching threads, or coming back to one puts
  // focus in the prompt, ready to type.
  $effect(() => {
    void draftId
    return nav.onArrival(takeFocusSoon)
  })

  // The conversation stays mounted under the slide-outs, so closing one (or picking the
  // thread tab again) brings focus back here rather than to a fresh composer.
  $effect(() => {
    if (workspaceUi.activeTab === 'thread') takeFocusSoon()
  })

  // Coming back to the app with nothing focused lands in the prompt too.
  function onWindowFocus() {
    if (document.activeElement === document.body) takeFocusSoon()
  }

  // A click on a control in the composer's footer (attach, New session, pause) leaves
  // the prompt focused; a menu that opens keeps focus until it closes.
  function onFootClick(event: MouseEvent) {
    if ((event.target as Element).closest('button')) takeFocusSoon()
  }

  /** Typing while focus has drifted (after a click in the conversation) goes to the prompt. */
  function typeToFocus(event: KeyboardEvent) {
    if (
      event.defaultPrevented ||
      event.metaKey ||
      event.ctrlKey ||
      event.altKey ||
      event.key.length !== 1 ||
      shellDialogs.anyOpen()
    )
      return
    const active = document.activeElement
    if (active === inputEl || isEditable(active)) return
    // Space presses a focused button; only redirect it from nowhere.
    if (event.key === ' ' && active !== document.body) return
    if (inputEl && composerMayTakeFocus(inputEl)) inputEl.focus()
  }

  function fitHeight() {
    if (!inputEl) return
    inputEl.style.height = 'auto'
    inputEl.style.height = `${Math.min(inputEl.scrollHeight, 200)}px`
  }

  function onInput(event: Event) {
    const target = event.currentTarget as HTMLTextAreaElement
    composerDrafts.setFor(draftId, target.value)
    caret = target.selectionStart
    historyIndex = null
    pickerDismissed = false
    pickerIndex = 0
    fitHeight()
  }

  async function sendMessage(stay = false) {
    const text = draft.trim()
    if (!canSend) return
    // The composer is reused across thread tabs, so pin the id before awaiting.
    const threadId = draftId
    const attached = attachments
    composerDrafts.setFor(threadId, '')
    if (attached.length) composerDrafts.setAttachments(threadId, [])
    historyIndex = null
    // Clearing the draft disables Send, and disabling a focused button (reached with Tab)
    // drops focus, so put it back on the prompt.
    void tick().then(() => {
      fitHeight()
      inputEl?.focus()
    })
    onSent?.()
    sendingDrafts.add(threadId)
    const outgoing = await expandMessage(text)
    const sent = await (
      newThread
        ? newThread.start(outgoing, stay, attached)
        : sendToThread(threadId, outgoing, attached)
    ).finally(() => sendingDrafts.delete(threadId))
    if (!sent) {
      composerDrafts.setFor(threadId, text)
      if (attached.length) composerDrafts.addAttachments(threadId, attached)
    }
  }

  async function attachFiles(files: File[]) {
    const threadId = draftId
    for (const file of files) {
      try {
        composerDrafts.addAttachments(threadId, [
          await attachmentFromFile(file),
        ])
      } catch (error) {
        toastCoreError(error, 'Could not attach')
      }
    }
  }

  function onPaste(event: ClipboardEvent) {
    const files = [...(event.clipboardData?.files ?? [])]
    if (!files.length) return
    event.preventDefault()
    void attachFiles(files)
  }

  function hasFiles(event: DragEvent) {
    return event.dataTransfer?.types.includes('Files') ?? false
  }

  /** Files from the picker or a Finder drop; images get their thumbnail from the core. */
  function attachPaths(paths: string[]) {
    const threadId = draftId
    const added = paths.map(attachmentFromPath)
    composerDrafts.addAttachments(threadId, added)
    for (const attachment of added) {
      if (!attachment.path || !isImageName(attachment.name)) continue
      void commands.imagePreview(attachment.path).then((result) => {
        if (result.status === 'ok' && result.data)
          composerDrafts.setPreview(threadId, attachment.id, result.data)
      })
    }
  }

  async function pickFiles() {
    const picked = await openFilePicker({ multiple: true, directory: false })
    const paths =
      picked == null ? [] : Array.isArray(picked) ? picked : [picked]
    if (paths.length) attachPaths(paths)
    inputEl?.focus()
  }

  // In the app, files dropped from Finder arrive as paths through Tauri, not as a browser drop.
  onMount(() => {
    let unlisten: (() => void) | undefined
    let gone = false
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === 'over' || event.payload.type === 'enter') {
          draggingFiles = true
        } else if (event.payload.type === 'leave') {
          draggingFiles = false
        } else if (event.payload.type === 'drop') {
          draggingFiles = false
          if (!event.payload.paths.length) return
          attachPaths(event.payload.paths)
        }
      })
      .then((stop) => {
        if (gone) stop()
        else unlisten = stop
      })
      .catch((error: unknown) =>
        toastCoreError(error, 'Files dropped from Finder can’t be attached'),
      )
    return () => {
      gone = true
      unlisten?.()
    }
  })

  function stopTurn() {
    if (!thread) return
    takeFocusSoon()
    void stopThread(thread.id)
  }

  function togglePause() {
    if (thread) void setThreadPaused(thread.id, !isPaused)
  }

  function onKeydown(event: KeyboardEvent) {
    // WebKit fires compositionend before the IME's confirming Enter keydown,
    // so `composing` alone misses it; isComposing / keyCode 229 catch it.
    const imeConfirm = composing || event.isComposing || event.keyCode === 229
    if (pickerOpen && !imeConfirm && onPickerKeydown(event)) return
    if (event.key === 'Enter' && !event.shiftKey && !imeConfirm) {
      event.preventDefault()
      void sendMessage(event.metaKey || event.ctrlKey)
      return
    }
    const plain =
      !event.shiftKey && !event.altKey && !event.metaKey && !event.ctrlKey
    if (
      plain &&
      !imeConfirm &&
      (event.key === 'ArrowUp' || event.key === 'ArrowDown')
    ) {
      recallPrompt(event)
    }
  }

  // ↑/↓ cycle past prompts from an empty composer, and keep cycling while the
  // recalled prompt is untouched and the caret is on its first (↑) or last (↓) line.
  function recallPrompt(event: KeyboardEvent) {
    const target = event.currentTarget as HTMLTextAreaElement
    const prompts = view?.promptHistory ?? []
    const older = event.key === 'ArrowUp'
    if (draft !== '') {
      if (historyIndex === null || draft !== prompts[historyIndex]) return
      const caret = target.selectionStart
      const atEdge = older
        ? !draft.slice(0, caret).includes('\n')
        : !draft.slice(target.selectionEnd).includes('\n')
      if (!atEdge) return
    }
    const next = stepPromptHistory(
      historyIndex,
      prompts.length,
      older ? 'older' : 'newer',
    )
    if (next === undefined) return
    event.preventDefault()
    historyIndex = next
    const text = next === null ? '' : (prompts[next] ?? '')
    composerDrafts.setFor(draftId, text)
    void tick().then(() => {
      fitHeight()
      target.setSelectionRange(text.length, text.length)
    })
  }

  function onCompositionStart() {
    composing = true
  }

  function onCompositionEnd() {
    composing = false
  }
</script>

<svelte:window onkeydown={typeToFocus} onfocus={onWindowFocus} />

<div
  class="composer-wrap sticky bottom-0 z-10 pt-2 pb-4 pl-1"
  data-od-id="composer-wrap"
>
  {#if status && !pickerOpen}
    <div
      class="pointer-events-none absolute inset-x-1 bottom-full mx-auto mb-1 flex max-w-(--thread-column)"
    >
      {@render status()}
    </div>
  {/if}
  {#if pickerOpen}
    <!-- Solid, not glass: it sits over the greeting's big type, which would show through. -->
    {@const shownKind = pickerOptions[0]?.kind ?? 'skill'}
    <ul
      id="composer-picker"
      role="listbox"
      aria-label={shownKind === 'repo'
        ? 'Repositories'
        : shownKind === 'task'
          ? 'Sprint tasks'
          : shownKind === 'bang'
            ? 'Bangs'
            : 'Skills'}
      class="composer-picker absolute inset-x-1 bottom-full mx-auto mb-1 flex max-w-(--thread-column) flex-col gap-0.5 rounded-xl border border-border/80 bg-background-solid p-1 text-popover-foreground shadow-lg"
      data-od-id="{shownKind}-picker"
    >
      {#each pickerOptions as option, index (option.key)}
        <li
          id="composer-option-{index}"
          role="option"
          aria-selected={index === pickerIndex}
          class="flex min-w-0 cursor-default items-center gap-2 rounded-lg px-2 py-1.5 text-sm aria-selected:bg-muted"
          onmousedown={(event) => {
            event.preventDefault()
            option.choose(option)
          }}
          onpointermove={(event) => {
            // Only a pointer that really moved: the list opening, or sliding in
            // under a still one, mustn't steal the highlight from the keyboard.
            if (event.movementX || event.movementY) pickerIndex = index
          }}
        >
          {#if option.kind === 'bang'}
            <Zap
              class="size-3.5 shrink-0 text-muted-foreground"
              aria-hidden="true"
            />
          {:else if option.kind === 'task'}
            <Kanban
              class="size-3.5 shrink-0 text-muted-foreground"
              aria-hidden="true"
            />
          {:else if option.kind === 'repo'}
            <Folder
              class="size-3.5 shrink-0 text-muted-foreground"
              aria-hidden="true"
            />
          {:else}
            <Sparkles
              class="size-3.5 shrink-0 text-muted-foreground"
              aria-hidden="true"
            />
          {/if}
          <span class="shrink-0 font-mono text-xs">{option.label}</span>
          {#if option.detail}
            <span class="min-w-0 truncate text-xs text-muted-foreground"
              >{option.detail}</span
            >
          {/if}
          {#if option.badge}
            <span
              class="ml-auto shrink-0 rounded border border-border/60 px-1 text-[10px] text-muted-foreground"
              >{option.badge}</span
            >
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
  <form
    class="composer glass-pane @container mx-auto flex max-w-(--thread-column) flex-col overflow-hidden rounded-[18px] transition-[border-color,box-shadow] duration-(--dur-fast) focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/25"
    id="composer"
    data-od-id="composer"
    class:composer-dropping={draggingFiles}
    onsubmit={(event) => {
      event.preventDefault()
      void sendMessage()
    }}
    ondragover={(event) => {
      if (!hasFiles(event)) return
      event.preventDefault()
      draggingFiles = true
    }}
    ondragleave={() => (draggingFiles = false)}
    ondrop={(event) => {
      if (!hasFiles(event)) return
      event.preventDefault()
      draggingFiles = false
      void attachFiles([...(event.dataTransfer?.files ?? [])])
    }}
  >
    {#if attachments.length}
      <ul
        class="flex flex-wrap gap-3 px-4 pt-4 pb-3"
        aria-label="Attachments"
        data-od-id="composer-attachments"
      >
        {#each attachments as attachment (attachment.id)}
          {#if attachment.previewUrl}
            <li
              class="group relative"
              title={attachment.path ?? attachment.name}
              data-od-id="composer-attachment"
            >
              <img
                src={attachment.previewUrl}
                alt={attachment.name}
                class="size-16 rounded-lg border border-border object-cover"
              />
              <button
                type="button"
                class="absolute -top-1.5 -right-1.5 flex size-5 items-center justify-center rounded-full border border-border bg-background text-muted-foreground shadow-sm hover:text-foreground"
                aria-label="Remove {attachment.name}"
                onclick={() =>
                  composerDrafts.removeAttachment(draftId, attachment.id)}
              >
                <X class="size-3" aria-hidden="true" />
              </button>
            </li>
          {:else}
            <li
              class="flex h-16 max-w-56 items-center gap-2 self-end rounded-lg border border-border bg-muted/40 p-1.5 pr-2.5 text-xs"
              title={attachment.path ?? attachment.name}
              data-od-id="composer-attachment"
            >
              <span
                class="flex size-9 shrink-0 items-center justify-center rounded bg-muted"
              >
                <FileText
                  class="size-4 text-muted-foreground"
                  aria-hidden="true"
                />
              </span>
              <span class="truncate">{attachment.name}</span>
              <button
                type="button"
                class="flex size-5 shrink-0 items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground"
                aria-label="Remove {attachment.name}"
                onclick={() =>
                  composerDrafts.removeAttachment(draftId, attachment.id)}
              >
                <X class="size-3" aria-hidden="true" />
              </button>
            </li>
          {/if}
        {/each}
      </ul>
    {/if}
    <label class="sr-only" for="composer-input"
      >{thread ? 'Message the agent' : 'Start a new thread'}</label
    >
    <Textarea
      bind:ref={inputEl}
      id="composer-input"
      data-od-id="composer-input"
      rows={1}
      autocomplete="off"
      {placeholder}
      class="max-h-[200px] min-h-[48px] resize-none border-0 bg-transparent px-4 pt-3 pb-1 shadow-none focus-visible:ring-0"
      value={draft}
      aria-controls={pickerOpen ? 'composer-picker' : undefined}
      aria-activedescendant={pickerOpen
        ? `composer-option-${pickerIndex}`
        : undefined}
      aria-autocomplete="list"
      oninput={onInput}
      onkeydown={onKeydown}
      onkeyup={syncCaret}
      onclick={syncCaret}
      onselect={syncCaret}
      oncompositionstart={onCompositionStart}
      oncompositionend={onCompositionEnd}
      onpaste={onPaste}
    />

    <div
      class="composer-foot flex items-center gap-2 px-2 py-2 pl-3"
      data-od-id="composer-foot"
      role="presentation"
      onclick={onFootClick}
    >
      <div
        class="composer-meta flex min-w-0 flex-1 items-center gap-2 text-xs text-muted-foreground"
        id="composer-meta"
        data-od-id="composer-meta"
      >
        <!-- A new thread brings its own model and effort pickers: there's no thread to set yet. -->
        {#if thread}
          <ThreadOptionPicker kind="model" threadId={thread.id} />
          <ThreadOptionPicker kind="effort" threadId={thread.id} />
        {:else if newThread}
          {@render newThread.pickers()}
        {/if}
        <Button
          type="button"
          variant="ghost"
          size="icon-xs"
          class="text-muted-foreground"
          data-od-id="composer-attach"
          aria-label="Attach images or files"
          title="Attach images or files (you can also paste or drop them)"
          onclick={() => void pickFiles()}
        >
          <Paperclip class="size-3.5" aria-hidden="true" />
        </Button>
        {#if contextFill != null}
          <span
            class="shrink-0 font-mono text-[10px] text-muted-foreground/80 tabular-nums"
            data-od-id="composer-usage"
            title={usageTooltip}
            aria-label="Context {contextFill}% full">{contextFill}%</span
          >
        {/if}
        {#if showPause}
          <Button
            type="button"
            variant="ghost"
            size="xs"
            data-od-id="composer-pause"
            aria-label={pauseLabel}
            onclick={togglePause}
          >
            {#if isPaused}
              <Play class="size-3.5" aria-hidden="true" />
            {:else}
              <Pause class="size-3.5" aria-hidden="true" />
            {/if}
            {pauseLabel}
          </Button>
        {/if}
        {#if thread}
          <Button
            type="button"
            variant="ghost"
            size="xs"
            data-od-id="composer-new-session"
            title="Start a new session; the agent forgets this conversation"
            disabled={!view?.canClearSession}
            onclick={() => void switchConversation(thread.id, 'new')}
          >
            New session
            <kbd
              class="rounded border border-border/60 px-1 text-[10px] text-muted-foreground/80"
              aria-hidden="true">{shortcutLabel('newSession')}</kbd
            >
          </Button>
        {/if}
      </div>

      <!-- Beside a pinned pane the bar is narrow; the chips go before they'd overlap the buttons. -->
      <span
        class="composer-hint hidden text-[10px] text-muted-foreground/80 @[34rem]:inline"
        aria-hidden="true"
      >
        <kbd class="rounded border border-border/60 px-1">/</kbd> skills
        {#if repos.items.length}
          <kbd class="rounded border border-border/60 px-1">@</kbd> repos
        {/if}
        {#if sprintTasks.length}
          <kbd class="rounded border border-border/60 px-1">#</kbd> tasks
        {/if}
        <kbd class="rounded border border-border/60 px-1">!</kbd> bangs
      </span>

      {#if threadStatus === 'running'}
        <Button
          type="button"
          variant="secondary"
          size="icon-sm"
          class="size-8 shrink-0 rounded-lg"
          data-od-id="composer-stop"
          title="Stop (Esc)"
          aria-label="Stop"
          onclick={stopTurn}
        >
          <Square class="size-3 fill-current" aria-hidden="true" />
        </Button>
      {/if}

      <Button
        type="submit"
        size="icon-sm"
        class="composer-send size-8 shrink-0 rounded-lg"
        data-od-id="composer-send"
        aria-label={thread ? 'Send to agent' : 'Start thread'}
        aria-busy={sending}
        disabled={!canSend}
        onmousedown={(event) => {
          // Keep the prompt focused; the click still submits.
          event.preventDefault()
        }}
      >
        {#if sending}
          <LoaderCircle class="size-4 animate-spin" aria-hidden="true" />
        {:else}
          <ArrowUp class="size-4" aria-hidden="true" />
        {/if}
      </Button>
    </div>
  </form>
</div>

<style>
  /* A pane docked beside the conversation (ThreadPanel's references) sets the gutter. */
  .composer-wrap {
    padding-right: var(--references-gutter, calc(var(--spacing) * 1));
  }

  .composer-picker {
    transform-origin: bottom center;
    animation: composer-picker-in var(--dur-med) var(--ease-out-snappy);
  }

  @keyframes composer-picker-in {
    from {
      opacity: 0;
      transform: translate3d(0, 6px, 0) scale(0.98);
    }
  }
</style>
