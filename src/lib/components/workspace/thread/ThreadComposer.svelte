<script lang="ts">
  import { Button } from '$lib/components/ui/button'
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
  import { filterSkills, skillQuery } from '$lib/composer/skill-picker'
  import { showToast } from '$lib/feedback/show-toast'
  import { composerDrafts } from '$lib/state/composer-drafts.svelte'
  import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
  import { sendThreadMessage } from '$lib/thread/send-message'
  import {
    hasSessionToClear,
    NEW_SESSION_SHORTCUT,
  } from '$lib/thread/new-session'
  import { startNewSession } from '$lib/thread/start-new-session'
  import { threads } from '$lib/state/threads.svelte'
  import { threadTimeline } from '$lib/state/thread-timeline.svelte'
  import type { Thread } from '$lib/state/threads.svelte'
  import Buddy from '$lib/components/buddy/Buddy.svelte'
  import { agentName } from '$lib/agent-name'
  import { buddyColor, buddyMoodForThread } from '$lib/buddy'
  import { engineDisplayName } from '$lib/sidebar/engine'
  import ThreadModelPicker from './ThreadModelPicker.svelte'
  import ArrowUp from '@lucide/svelte/icons/arrow-up'
  import Pause from '@lucide/svelte/icons/pause'
  import Play from '@lucide/svelte/icons/play'
  import Sparkles from '@lucide/svelte/icons/sparkles'
  import Square from '@lucide/svelte/icons/square'

  let {
    thread,
    onSent,
    // eslint-disable-next-line no-useless-assignment -- bindable, set by the effect below
    focusComposer = $bindable(),
    inputLabel = 'Message the agent',
    placeholder,
    sendLabel = 'Send to agent',
    newSession = false,
  }: {
    thread: Thread
    onSent?: () => void
    focusComposer?: (() => void) | null
    inputLabel?: string
    placeholder?: string
    sendLabel?: string
    /** Show the New session button (threads only; scratches end instead). */
    newSession?: boolean
  } = $props()

  let inputEl = $state<HTMLTextAreaElement | null>(null)
  let composing = $state(false)
  /** Which past prompt ↑/↓ recalled into the composer; null when not cycling. */
  let historyIndex = $state<number | null>(null)

  const mood = $derived(buddyMoodForThread(thread))
  const color = $derived(buddyColor(thread.id))
  const name = $derived(agentName(thread.id))
  const engineName = $derived(engineDisplayName(thread.engine))
  const draft = $derived(composerDrafts.textFor(thread.id))
  const canSend = $derived(draft.trim().length > 0)
  const showPause = $derived(composerShowsPauseControl(thread.status))
  const pauseLabel = $derived(composerPauseLabel(thread.status, thread.paused))
  const isPaused = $derived(thread.status === 'paused' || thread.paused)
  const canStartNewSession = $derived(
    hasSessionToClear(threadTimeline.eventsByThread[thread.id] ?? []),
  )

  // Typing `/` opens the skill picker. Skills are Claude Code's, so other engines skip it.
  let skills = $state<Skill[] | null>(null)
  let skillsThreadId: string | null = null
  let skillIndex = $state(0)
  /** Esc hides the picker until the draft changes. */
  let skillPickerDismissed = $state(false)
  const query = $derived(thread.engine === 'claude' ? skillQuery(draft) : null)
  const skillMatches = $derived(
    query !== null && skills ? filterSkills(skills, query) : [],
  )
  const pickerOpen = $derived(
    query !== null && !skillPickerDismissed && skillMatches.length > 0,
  )

  $effect(() => {
    void thread.id
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
    void commands.listThreadSkills(threadId).then((result) => {
      if (thread.id !== threadId) return
      skills = result.status === 'ok' ? result.data : []
    })
  })

  function chooseSkill(skill: Skill) {
    const text = `/${skill.name} `
    composerDrafts.setFor(thread.id, text)
    skillIndex = 0
    queueMicrotask(() => {
      fitHeight()
      inputEl?.focus()
      inputEl?.setSelectionRange(text.length, text.length)
    })
  }

  // Arrows move through the picker, ↵ or ⇥ picks, Esc closes it. Returns whether it handled the key.
  function onPickerKeydown(event: KeyboardEvent): boolean {
    const count = skillMatches.length
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      const step = event.key === 'ArrowDown' ? 1 : -1
      skillIndex = (skillIndex + step + count) % count
    } else if (
      (event.key === 'Enter' && !event.shiftKey) ||
      event.key === 'Tab'
    ) {
      const skill = skillMatches[skillIndex]
      if (skill) chooseSkill(skill)
    } else if (event.key === 'Escape') {
      skillPickerDismissed = true
    } else {
      return false
    }
    event.preventDefault()
    return true
  }

  $effect(() => {
    focusComposer = () => {
      inputEl?.focus()
    }
  })

  function fitHeight() {
    if (!inputEl) return
    inputEl.style.height = 'auto'
    inputEl.style.height = `${Math.min(inputEl.scrollHeight, 200)}px`
  }

  function onInput(event: Event) {
    const target = event.currentTarget as HTMLTextAreaElement
    composerDrafts.setFor(thread.id, target.value)
    historyIndex = null
    skillPickerDismissed = false
    skillIndex = 0
    fitHeight()
  }

  async function sendMessage() {
    const text = draft.trim()
    if (!text) return
    // The composer is reused across thread tabs, so pin the id before awaiting.
    const threadId = thread.id
    composerDrafts.setFor(threadId, '')
    historyIndex = null
    // Clearing the draft disables Send. A click focuses that button first, and
    // disabling a focused button drops focus, so put it back on the prompt.
    queueMicrotask(() => {
      fitHeight()
      inputEl?.focus()
    })
    onSent?.()
    if (!(await sendThreadMessage(threadId, text))) {
      composerDrafts.setFor(threadId, text)
    }
  }

  async function stopTurn() {
    threads.setStatus(thread.id, 'idle')
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
      void sendMessage()
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

<svelte:window onkeydown={onWindowKeydown} />

<div
  class="composer-wrap sticky bottom-0 z-10 bg-background/95 px-1 py-3 backdrop-blur supports-[backdrop-filter]:bg-background/80"
  data-od-id="composer-wrap"
>
  {#if pickerOpen}
    <ul
      id="skill-picker"
      role="listbox"
      aria-label="Skills"
      class="absolute inset-x-1 bottom-full mx-auto -mb-1 flex max-w-[760px] flex-col gap-0.5 rounded-xl border border-border/80 bg-popover p-1 text-popover-foreground shadow-lg"
      data-od-id="skill-picker"
    >
      {#each skillMatches as skill, index (skill.name)}
        <li
          id="skill-option-{index}"
          role="option"
          aria-selected={index === skillIndex}
          class="flex min-w-0 cursor-default items-center gap-2 rounded-lg px-2 py-1.5 text-sm aria-selected:bg-muted"
          onmousedown={(event) => {
            // Keep focus in the composer.
            event.preventDefault()
            chooseSkill(skill)
          }}
          onmouseenter={() => (skillIndex = index)}
        >
          <Sparkles
            class="size-3.5 shrink-0 text-muted-foreground"
            aria-hidden="true"
          />
          <span class="shrink-0 font-mono text-xs">/{skill.name}</span>
          {#if skill.description}
            <span class="min-w-0 truncate text-xs text-muted-foreground"
              >{skill.description}</span
            >
          {/if}
          {#if skill.source === 'project' || skill.source === 'plugin'}
            <span
              class="ml-auto shrink-0 rounded border border-border/60 px-1 text-[10px] text-muted-foreground"
              >{skill.source === 'project' ? 'repo' : 'plugin'}</span
            >
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
  <form
    class="composer mx-auto flex max-w-[760px] flex-col overflow-hidden rounded-xl border border-border/80 bg-background shadow-[0_-12px_32px_-8px_var(--background)] focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/30"
    id="composer"
    data-od-id="composer"
    onsubmit={(event) => {
      event.preventDefault()
      void sendMessage()
    }}
  >
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
      aria-controls={pickerOpen ? 'skill-picker' : undefined}
      aria-activedescendant={pickerOpen
        ? `skill-option-${skillIndex}`
        : undefined}
      aria-autocomplete={thread.engine === 'claude' ? 'list' : undefined}
      oninput={onInput}
      onkeydown={onKeydown}
      oncompositionstart={onCompositionStart}
      oncompositionend={onCompositionEnd}
    />

    <div
      class="composer-foot flex items-center gap-2 px-2 py-2 pl-3"
      data-od-id="composer-foot"
    >
      <div
        class="composer-meta flex min-w-0 flex-1 items-center gap-2 text-xs text-muted-foreground"
        id="composer-meta"
        data-od-id="composer-meta"
      >
        <Buddy {mood} {color} glasses={thread.role === 'Reviewer'} size={20} />
        <span class="lbl truncate"
          >{thread.role ? `${engineName} · ${name}` : engineName}</span
        >
        <ThreadModelPicker {thread} />
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
        <kbd class="rounded border border-border/60 px-1">↵</kbd> send
        <kbd class="rounded border border-border/60 px-1">⇧↵</kbd> new line
        {#if thread.engine === 'claude'}
          <kbd class="rounded border border-border/60 px-1">/</kbd> skills
        {/if}
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
