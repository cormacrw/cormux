<!-- A thread's own controls in the composer's footer: how full its context is, pause, and a new session. -->
<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import {
    composerPauseLabel,
    composerShowsPauseControl,
  } from '$lib/composer/can-pause'
  import { shortcutLabel } from '$lib/keyboard/shortcuts'
  import type { Thread } from '$lib/state/threads.svelte'
  import { setThreadPaused, switchConversation } from '$lib/thread/actions'
  import { contextPercent, usageDetail } from '$lib/usage/usage'
  import Pause from '@lucide/svelte/icons/pause'
  import Play from '@lucide/svelte/icons/play'

  let { thread }: { thread: Thread } = $props()

  const showPause = $derived(composerShowsPauseControl(thread.status))
  const pauseLabel = $derived(composerPauseLabel(thread.status))
  const isPaused = $derived(thread.status === 'paused')
  // Only how full the context is; tokens and cost are in the thread tab's tooltip.
  const contextFill = $derived(contextPercent(thread.usage))
</script>

{#if contextFill != null}
  <span
    class="shrink-0 font-mono text-[10px] text-muted-foreground/80 tabular-nums"
    data-od-id="composer-usage"
    title={usageDetail(thread.usage)}
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
    onclick={() => void setThreadPaused(thread.id, !isPaused)}
  >
    {#if isPaused}
      <Play class="size-3.5" aria-hidden="true" />
    {:else}
      <Pause class="size-3.5" aria-hidden="true" />
    {/if}
    {pauseLabel}
  </Button>
{/if}
<Button
  type="button"
  variant="ghost"
  size="xs"
  data-od-id="composer-new-session"
  title="Start a new session; the agent forgets this conversation"
  disabled={!thread.hasSession}
  onclick={() => void switchConversation(thread.id, 'new')}
>
  New session
  <kbd
    class="rounded border border-border/60 px-1 text-[10px] text-muted-foreground/80"
    aria-hidden="true">{shortcutLabel('newSession')}</kbd
  >
</Button>
