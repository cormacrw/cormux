<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { Textarea } from '$lib/components/ui/textarea'
  import {
    composerPauseLabel,
    composerShowsPauseControl,
  } from '$lib/composer/can-pause'
  import { commands } from '$lib/ipc'
  import { showToast } from '$lib/feedback/show-toast'
  import { composerDrafts } from '$lib/state/composer-drafts.svelte'
  import { sendThreadMessage } from '$lib/thread/send-message'
  import {
    hasSessionToClear,
    NEW_SESSION_SHORTCUT,
  } from '$lib/thread/new-session'
  import { startNewSession } from '$lib/thread/start-new-session'
  import { threads } from '$lib/state/threads.svelte'
  import { threadTimeline } from '$lib/state/thread-timeline.svelte'
  import type { Thread } from '$lib/state/threads.svelte'
  import { engineDisplayName, engineMark } from '$lib/sidebar/engine'
  import ArrowUp from '@lucide/svelte/icons/arrow-up'
  import Pause from '@lucide/svelte/icons/pause'
  import Play from '@lucide/svelte/icons/play'
  import Square from '@lucide/svelte/icons/square'

  let {
    thread,
    onSent,
    focusComposer = $bindable<(() => void) | null>(null),
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

  const mark = $derived(engineMark(thread.engine))
  const engineName = $derived(engineDisplayName(thread.engine))
  const draft = $derived(composerDrafts.textFor(thread.id))
  const canSend = $derived(draft.trim().length > 0)
  const showPause = $derived(composerShowsPauseControl(thread.status))
  const pauseLabel = $derived(
    composerPauseLabel(thread.status, thread.paused),
  )
  const isPaused = $derived(thread.status === 'paused' || thread.paused)
  const canStartNewSession = $derived(
    hasSessionToClear(threadTimeline.eventsByThread[thread.id] ?? []),
  )

  $effect(() => {
    void thread.id
    queueMicrotask(() => fitHeight())
  })

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
    fitHeight()
  }

  async function sendMessage() {
    const text = draft.trim()
    if (!text) return
    // The composer is reused across thread tabs, so pin the id before awaiting.
    const threadId = thread.id
    composerDrafts.setFor(threadId, '')
    queueMicrotask(() => fitHeight())
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
    if (event.key === 'Enter' && !event.shiftKey && !imeConfirm) {
      event.preventDefault()
      void sendMessage()
    }
  }

  function onCompositionStart() {
    composing = true
  }

  function onCompositionEnd() {
    composing = false
  }
</script>

<div
  class="composer-wrap sticky bottom-0 z-10 border-t border-border/70 bg-background/95 px-1 py-3 backdrop-blur supports-[backdrop-filter]:bg-background/80"
  data-od-id="composer-wrap"
>
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
      placeholder={placeholder ?? `Message ${thread.role}…`}
      class="max-h-[200px] min-h-[48px] resize-none border-0 bg-transparent px-4 pt-3 pb-1 shadow-none focus-visible:ring-0"
      value={draft}
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
        <span
          class="engine-mark flex size-5 shrink-0 items-center justify-center rounded bg-muted font-mono text-[9px] font-semibold"
          aria-hidden="true">{mark}</span
        >
        <span class="lbl truncate"
          >{thread.role ? `${engineName} · ${thread.role}` : engineName}</span
        >
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

      {#if thread.status === 'running'}
        <Button
          type="button"
          variant="ghost"
          size="icon-xs"
          data-od-id="composer-stop"
          title="Stop"
          aria-label="Stop"
          onclick={() => void stopTurn()}
        >
          <Square class="size-3" />
        </Button>
      {/if}

      <span class="composer-hint text-[10px] text-muted-foreground/80" aria-hidden="true">
        <kbd class="rounded border border-border/60 px-1">↵</kbd> send
        <kbd class="rounded border border-border/60 px-1">⇧↵</kbd> new line
      </span>

      <Button
        type="submit"
        size="icon-sm"
        class="composer-send size-8 shrink-0 rounded-lg"
        data-od-id="composer-send"
        aria-label={sendLabel}
        disabled={!canSend}
      >
        <ArrowUp class="size-4" aria-hidden="true" />
      </Button>
    </div>
  </form>
</div>
