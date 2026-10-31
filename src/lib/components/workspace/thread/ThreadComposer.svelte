<script lang="ts">
  import type { Snippet } from 'svelte'
  import { onMount, tick } from 'svelte'
  import { composerMayTakeFocus, isEditable } from '$lib/keyboard/focus-home'
  import { app } from '$lib/state/app.svelte'
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
  import {
    promptHistory,
    stepPromptHistory,
  } from '$lib/composer/prompt-history'
  import { commands } from '$lib/ipc'
  import type { Skill } from '$lib/ipc/bindings'
  import {
    filterSkills,
    insertSkill,
    skillToken,
  } from '$lib/composer/skill-picker'
  import {
    filterRepos,
    insertRepoAlias,
    repoAlias,
    repoToken,
  } from '$lib/composer/repo-aliases'
  import {
    filterTasks,
    insertTaskAlias,
    taskToken,
  } from '$lib/composer/task-aliases'
  import { expandMessage } from '$lib/composer/expand-message'
  import {
    bangPreview,
    bangToken,
    filterBangs,
    insertBang,
    type Bang,
  } from '$lib/settings/bangs'
  import Zap from '@lucide/svelte/icons/zap'
  import { taskRef } from '$lib/clickup/board'
  import { clickup } from '$lib/state/clickup.svelte'
  import { repos } from '$lib/state/repos.svelte'
  import type { RepoRecord } from '$lib/ipc/bindings'
  import { showToast } from '$lib/feedback/show-toast'
  import { composerDrafts } from '$lib/state/composer-drafts.svelte'
  import { settings } from '$lib/state/settings.svelte'
  import { contextPercent, usageSummary } from '$lib/usage/usage'
  import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
  import { sendThreadMessage } from '$lib/thread/send-message'
  import {
    hasSessionToClear,
    NEW_SESSION_SHORTCUT,
  } from '$lib/thread/new-session'
  import { startNewSession } from '$lib/thread/start-new-session'
  import { threads } from '$lib/state/threads.svelte'
  import { threadTimeline } from '$lib/state/thread-timeline.svelte'
  import { workspaceUi } from '$lib/state/workspace-ui.svelte'
  import type { Thread } from '$lib/state/threads.svelte'
  import { agentName } from '$lib/agent-name'
  import ThreadModelPicker from './ThreadModelPicker.svelte'
  import ArrowUp from '@lucide/svelte/icons/arrow-up'
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
    thread,
    onSent,
    status,
    // eslint-disable-next-line no-useless-assignment -- bindable, set by the effect below
    focusComposer = $bindable(),
    inputLabel = 'Message the agent',
    placeholder,
    sendLabel = 'Send to agent',
    newSession = false,
    onSubmit,
    pickers,
    listSkills,
    disabled = false,
  }: {
    thread: Thread
    onSent?: () => void
    /** What the agent is doing, floated just above the prompt bar. */
    status?: Snippet
    focusComposer?: (() => void) | null
    inputLabel?: string
    placeholder?: string
    sendLabel?: string
    /** Show the New session button (a thread that's already running). */
    newSession?: boolean
    /**
     * Takes the message instead of the thread, for a composer whose thread doesn't exist yet.
     * Resolves false to put the draft back. `stay` is ⌘↵: send it off without leaving.
     */
    onSubmit?: (text: string, stay: boolean) => Promise<boolean>
    /** Controls for the start of the footer, before anything else. */
    pickers?: Snippet
    /** Reads the `/` picker's skills, for a composer whose thread doesn't exist yet. */
    listSkills?: () => Promise<Skill[]>
    disabled?: boolean
  } = $props()

  let inputEl = $state<HTMLTextAreaElement | null>(null)
  let composing = $state(false)
  /** Which past prompt ↑/↓ recalled into the composer; null when not cycling. */
  let historyIndex = $state<number | null>(null)

  const name = $derived(agentName(thread.id))
  const draft = $derived(composerDrafts.textFor(thread.id))
  const showPause = $derived(composerShowsPauseControl(thread.status))
  const pauseLabel = $derived(composerPauseLabel(thread.status, thread.paused))
  const isPaused = $derived(thread.status === 'paused' || thread.paused)
  // Only how full the context is; tokens and cost are in the thread tab's tooltip.
  const contextFill = $derived(contextPercent(thread.usage))
  const usageDetail = $derived(usageSummary(thread.usage).detail)
  const canStartNewSession = $derived(
    hasSessionToClear(threadTimeline.eventsByThread[thread.id] ?? []),
  )
  const attachments = $derived(composerDrafts.attachmentsFor(thread.id))
  const canSend = $derived(
    !disabled && (draft.trim().length > 0 || attachments.length > 0),
  )
  // No thread yet means no model to pick and nowhere to keep attachments.
  const canAttach = $derived(!onSubmit)
  let draggingFiles = $state(false)

  // Typing `/` anywhere opens the skill picker. Skills are Claude Code's, so other engines skip it.
  let skills = $state<Skill[] | null>(null)
  let skillsThreadId: string | null = null
  let pickerIndex = $state(0)
  /** Esc hides the picker until the draft changes. */
  let pickerDismissed = $state(false)
  /** Where the caret is in the draft; the picker follows the `/` or `@` word it's in. */
  let caret = $state(0)
  const token = $derived(
    thread.engine === 'claude'
      ? skillToken(draft, Math.min(caret, draft.length))
      : null,
  )
  const query = $derived(token?.query ?? null)
  const skillMatches = $derived(
    query !== null && skills ? filterSkills(skills, query) : [],
  )
  // Typing `@` offers the repos set up in Cormux; `@alias` becomes the repo's folder on send.
  const repoWord = $derived(
    token ? null : repoToken(draft, Math.min(caret, draft.length)),
  )
  const repoMatches = $derived(
    repoWord ? filterRepos(repos.items, repoWord.query) : [],
  )
  // Typing `#` offers your sprint tasks; `#ref` becomes the task on send.
  const sprintTasks = $derived(clickup.mine?.tasks ?? [])
  const taskWord = $derived(
    token || repoWord || !sprintTasks.length
      ? null
      : taskToken(draft, Math.min(caret, draft.length)),
  )
  const taskMatches = $derived(
    taskWord ? filterTasks(sprintTasks, taskWord.query) : [],
  )
  // Typing `!` offers your bangs (Settings › Bangs); picking one puts its prompt in.
  const bangWord = $derived(
    token || repoWord || taskWord || !settings.bangs.length
      ? null
      : bangToken(draft, Math.min(caret, draft.length)),
  )
  const bangMatches = $derived(
    bangWord ? filterBangs(settings.bangs, bangWord.query) : [],
  )
  type PickerOption = {
    key: string
    label: string
    detail: string
    badge: string | null
    kind: 'skill' | 'repo' | 'task' | 'bang'
    choose: () => void
  }
  const pickerOptions = $derived.by((): PickerOption[] => {
    if (query !== null)
      return skillMatches.map((skill) => ({
        key: skill.name,
        label: `/${skill.name}`,
        detail: skill.description,
        badge:
          skill.source === 'project'
            ? 'repo'
            : skill.source === 'plugin'
              ? 'plugin'
              : null,
        kind: 'skill' as const,
        choose: () => chooseSkill(skill),
      }))
    if (bangWord)
      return bangMatches.map((bang) => ({
        key: bang.id,
        label: `!${bang.name}`,
        detail: bangPreview(bang.prompt, 80),
        badge: null,
        kind: 'bang' as const,
        choose: () => chooseBang(bang),
      }))
    if (taskWord)
      return taskMatches.map((task) => ({
        key: task.id,
        label: `#${taskRef(task)}`,
        detail: task.name,
        badge: task.status,
        kind: 'task' as const,
        choose: () => chooseTask(task),
      }))
    return repoMatches.map((repo) => ({
      key: repo.id,
      label: `@${repoAlias(repo)}`,
      detail: repo.path,
      badge: null,
      kind: 'repo' as const,
      choose: () => chooseRepo(repo),
    }))
  })
  const pickerOpen = $derived(!pickerDismissed && pickerOptions.length > 0)

  $effect(() => {
    void thread.id
    void listSkills
    historyIndex = null
    skills = null
    skillsThreadId = null
    queueMicrotask(() => fitHeight())
  })

  // Read the skill folders when the picker is first wanted, once per thread.
  $effect(() => {
    if (query === null || skillsThreadId === thread.id) return
    const threadId = thread.id
    skillsThreadId = threadId
    if (listSkills) {
      void listSkills().then((found) => (skills = found))
      return
    }
    void commands.listThreadSkills(threadId).then((result) => {
      if (thread.id !== threadId) return
      skills = result.status === 'ok' ? result.data : []
    })
  })

  function chooseSkill(skill: Skill) {
    if (!token) return
    applyPick(insertSkill(draft, token, skill.name))
  }

  function chooseRepo(repo: RepoRecord) {
    if (!repoWord) return
    applyPick(insertRepoAlias(draft, repoWord, repoAlias(repo)))
  }

  function chooseBang(bang: Bang) {
    if (!bangWord) return
    applyPick(insertBang(draft, bangWord, bang))
  }

  function chooseTask(task: (typeof sprintTasks)[number]) {
    if (!taskWord) return
    applyPick(insertTaskAlias(draft, taskWord, taskRef(task)))
  }

  function applyPick(next: { text: string; caret: number }) {
    composerDrafts.setFor(thread.id, next.text)
    caret = next.caret
    pickerIndex = 0
    queueMicrotask(() => {
      fitHeight()
      inputEl?.focus()
      inputEl?.setSelectionRange(next.caret, next.caret)
    })
  }

  function syncCaret() {
    if (inputEl) caret = inputEl.selectionStart
  }

  // Arrows move through the picker, ↵ or ⇥ picks, Esc closes it. Returns whether it handled the key.
  function onPickerKeydown(event: KeyboardEvent): boolean {
    const count = pickerOptions.length
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      const step = event.key === 'ArrowDown' ? 1 : -1
      pickerIndex = (pickerIndex + step + count) % count
    } else if (
      (event.key === 'Enter' && !event.shiftKey) ||
      event.key === 'Tab'
    ) {
      pickerOptions[pickerIndex]?.choose()
    } else if (event.key === 'Escape') {
      pickerDismissed = true
    } else {
      return false
    }
    event.preventDefault()
    return true
  }

  $effect(() => {
    // Fits the box too, for text put in the draft from outside (a picked task).
    focusComposer = () => {
      fitHeight()
      inputEl?.focus()
    }
  })

  /** Puts focus back in the prompt, unless the user is busy somewhere else. */
  function takeFocus() {
    if (inputEl && composerMayTakeFocus(inputEl))
      inputEl.focus({ preventScroll: true })
  }

  // Opening a conversation, switching threads, or coming back to one puts
  // focus in the prompt, ready to type.
  $effect(() => {
    void thread.id
    void app.focusGeneration
    void tick().then(takeFocus)
  })

  // The conversation stays mounted under Git and Output, so closing them (or picking the
  // thread tab again) brings focus back here rather than to a fresh composer.
  $effect(() => {
    if (workspaceUi.activeTab !== 'thread') return
    void tick().then(takeFocus)
  })

  // Coming back to the app with nothing focused lands in the prompt too.
  function onWindowFocus() {
    if (document.activeElement === document.body) void tick().then(takeFocus)
  }

  // A click on a control in the composer's footer (attach, New session, pause) leaves
  // the prompt focused; a menu that opens keeps focus until it closes.
  function onFootClick(event: MouseEvent) {
    if ((event.target as Element).closest('button')) setTimeout(takeFocus, 0)
  }

  /** Typing while focus has drifted (after a click in the conversation) goes to the prompt. */
  function typeToFocus(event: KeyboardEvent) {
    if (
      event.defaultPrevented ||
      event.metaKey ||
      event.ctrlKey ||
      event.altKey ||
      event.key.length !== 1 ||
      shellDialogs.blocksCommandPalette()
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
    composerDrafts.setFor(thread.id, target.value)
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
    const threadId = thread.id
    const sending = attachments
    composerDrafts.setFor(threadId, '')
    if (sending.length) composerDrafts.setAttachments(threadId, [])
    historyIndex = null
    // Clearing the draft disables Send. A click focuses that button first, and
    // disabling a focused button drops focus, so put it back on the prompt.
    queueMicrotask(() => {
      fitHeight()
      inputEl?.focus()
    })
    onSent?.()
    const outgoing = await expandMessage(text)
    const sent = onSubmit
      ? await onSubmit(outgoing, stay)
      : await sendThreadMessage(threadId, outgoing, sending)
    if (!sent) {
      composerDrafts.setFor(threadId, text)
      if (sending.length) composerDrafts.addAttachments(threadId, sending)
    }
  }

  async function attachFiles(files: File[]) {
    const threadId = thread.id
    for (const file of files) {
      try {
        composerDrafts.addAttachments(threadId, [
          await attachmentFromFile(file),
        ])
      } catch (error) {
        showToast({
          tone: 'bad',
          parts: [
            {
              type: 'text',
              value:
                error instanceof Error ? error.message : 'Could not attach',
            },
          ],
        })
      }
    }
  }

  function onPaste(event: ClipboardEvent) {
    const files = [...(event.clipboardData?.files ?? [])]
    if (!files.length || !canAttach) return
    event.preventDefault()
    void attachFiles(files)
  }

  function hasFiles(event: DragEvent) {
    return event.dataTransfer?.types.includes('Files') ?? false
  }

  /** Files from the picker or a Finder drop; images get their thumbnail from the core. */
  function attachPaths(paths: string[]) {
    const threadId = thread.id
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
    if (document.documentElement.dataset.cormuxHarness === 'browser') return
    let unlisten: (() => void) | undefined
    let gone = false
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === 'over' || event.payload.type === 'enter') {
          draggingFiles = canAttach
        } else if (event.payload.type === 'leave') {
          draggingFiles = false
        } else if (event.payload.type === 'drop') {
          draggingFiles = false
          if (!canAttach || !event.payload.paths.length) return
          attachPaths(event.payload.paths)
        }
      })
      .then((stop) => {
        if (gone) stop()
        else unlisten = stop
      })
      .catch(() => {})
    return () => {
      gone = true
      unlisten?.()
    }
  })

  async function stopTurn() {
    threads.setStatus(thread.id, 'idle')
    void tick().then(takeFocus)
    threadTimeline.applyEvent(thread.id, {
      type: 'turnEnd',
      stop_reason: 'cancelled',
      error: null,
    })
    const result = await commands.cancelThreadTurn(thread.id)
    if (result.status === 'error') {
      showToast({
        tone: 'bad',
        parts: [
          {
            type: 'text',
            value:
              typeof result.error.message === 'string'
                ? result.error.message
                : 'Could not stop the agent',
          },
        ],
      })
    }
  }

  async function togglePause() {
    if (isPaused) {
      await commands.resumeThread(thread.id)
    } else {
      await commands.pauseThread(thread.id)
    }
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
    const prompts = promptHistory(
      threadTimeline.eventsByThread[thread.id] ?? [],
    )
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
    composerDrafts.setFor(thread.id, text)
    queueMicrotask(() => {
      fitHeight()
      target.setSelectionRange(text.length, text.length)
    })
  }

  // ⌘↵ anywhere jumps to the composer; dialogs and the diff comment box handle it first.
  function onWindowKeydown(event: KeyboardEvent) {
    typeToFocus(event)
    // Esc stops the agent, unless a dialog, popover or inline editor took it first.
    if (
      event.key === 'Escape' &&
      thread.status === 'running' &&
      !event.defaultPrevented &&
      !event.metaKey &&
      !event.ctrlKey &&
      !event.altKey &&
      !event.shiftKey &&
      !shellDialogs.blocksCommandPalette()
    ) {
      event.preventDefault()
      void stopTurn()
      return
    }
    if (
      event.key !== 'Enter' ||
      !(event.metaKey || event.ctrlKey) ||
      event.shiftKey ||
      event.altKey ||
      event.defaultPrevented ||
      shellDialogs.blocksCommandPalette()
    ) {
      return
    }
    event.preventDefault()
    inputEl?.focus()
  }

  function onCompositionStart() {
    composing = true
  }

  function onCompositionEnd() {
    composing = false
  }
</script>

<svelte:window onkeydown={onWindowKeydown} onfocus={onWindowFocus} />

<div
  class="composer-wrap sticky bottom-0 z-10 px-1 pt-2 pb-4"
  data-od-id="composer-wrap"
>
  {#if status && !pickerOpen}
    <div
      class="pointer-events-none absolute inset-x-1 bottom-full mx-auto mb-1 flex max-w-[760px]"
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
      class="composer-picker absolute inset-x-1 bottom-full mx-auto mb-1 flex max-w-[760px] flex-col gap-0.5 rounded-xl border border-border/80 bg-background-solid p-1 text-popover-foreground shadow-lg"
      data-od-id="{shownKind}-picker"
    >
      {#each pickerOptions as option, index (option.key)}
        <li
          id="composer-option-{index}"
          role="option"
          aria-selected={index === pickerIndex}
          class="flex min-w-0 cursor-default items-center gap-2 rounded-lg px-2 py-1.5 text-sm aria-selected:bg-muted"
          onmousedown={(event) => {
            // Keep focus in the composer.
            event.preventDefault()
            option.choose()
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
    class="composer glass-pane mx-auto flex max-w-[760px] flex-col overflow-hidden rounded-[18px] transition-[border-color,box-shadow] duration-(--dur-fast) focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/25"
    id="composer"
    data-od-id="composer"
    class:composer-dropping={draggingFiles}
    onsubmit={(event) => {
      event.preventDefault()
      void sendMessage()
    }}
    ondragover={(event) => {
      if (!hasFiles(event) || !canAttach) return
      event.preventDefault()
      draggingFiles = true
    }}
    ondragleave={() => (draggingFiles = false)}
    ondrop={(event) => {
      if (!hasFiles(event) || !canAttach) return
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
            <!-- An image is its thumbnail; the name is in the tooltip. -->
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
                  composerDrafts.removeAttachment(thread.id, attachment.id)}
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
                  composerDrafts.removeAttachment(thread.id, attachment.id)}
              >
                <X class="size-3" aria-hidden="true" />
              </button>
            </li>
          {/if}
        {/each}
      </ul>
    {/if}
    <label class="sr-only" for="composer-input">{inputLabel}</label>
    <Textarea
      bind:ref={inputEl}
      id="composer-input"
      data-od-id="composer-input"
      rows={1}
      autocomplete="off"
      placeholder={placeholder ?? `Message ${name}…`}
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
        {@render pickers?.()}
        {#if canAttach}
          <ThreadModelPicker {thread} />
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
        {/if}
        {#if contextFill != null}
          <span
            class="shrink-0 font-mono text-[10px] text-muted-foreground/80 tabular-nums"
            data-od-id="composer-usage"
            title={usageDetail}
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
            onclick={() => void togglePause()}
          >
            {#if isPaused}
              <Play class="size-3.5" aria-hidden="true" />
            {:else}
              <Pause class="size-3.5" aria-hidden="true" />
            {/if}
            {pauseLabel}
          </Button>
        {/if}
        {#if newSession}
          <Button
            type="button"
            variant="ghost"
            size="xs"
            data-od-id="composer-new-session"
            title="Start a new session; the agent forgets this conversation"
            disabled={!canStartNewSession}
            onclick={() => void startNewSession(thread.id)}
          >
            New session
            <kbd
              class="rounded border border-border/60 px-1 text-[10px] text-muted-foreground/80"
              aria-hidden="true">{NEW_SESSION_SHORTCUT}</kbd
            >
          </Button>
        {/if}
      </div>

      <span
        class="composer-hint text-[10px] text-muted-foreground/80"
        aria-hidden="true"
      >
        {#if thread.engine === 'claude'}
          <kbd class="rounded border border-border/60 px-1">/</kbd> skills
        {/if}
        {#if repos.items.length}
          <kbd class="rounded border border-border/60 px-1">@</kbd> repos
        {/if}
        {#if sprintTasks.length}
          <kbd class="rounded border border-border/60 px-1">#</kbd> tasks
        {/if}
        <kbd class="rounded border border-border/60 px-1">!</kbd> bangs
      </span>

      {#if thread.status === 'running'}
        <Button
          type="button"
          variant="secondary"
          size="icon-sm"
          class="size-8 shrink-0 rounded-lg"
          data-od-id="composer-stop"
          title="Stop (Esc)"
          aria-label="Stop"
          onclick={() => void stopTurn()}
        >
          <Square class="size-3 fill-current" aria-hidden="true" />
        </Button>
      {/if}

      <Button
        type="submit"
        size="icon-sm"
        class="composer-send size-8 shrink-0 rounded-lg"
        data-od-id="composer-send"
        aria-label={sendLabel}
        disabled={!canSend}
        onmousedown={(event) => {
          // Keep the prompt focused; the click still submits.
          event.preventDefault()
        }}
      >
        <ArrowUp class="size-4" aria-hidden="true" />
      </Button>
    </div>
  </form>
</div>

<style>
  /* The picker pops up from the prompt bar. */
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
