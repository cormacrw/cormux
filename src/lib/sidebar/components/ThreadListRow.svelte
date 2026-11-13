<script lang="ts">
  import { openWorkspace } from '$lib/nav/open'
  import { navigate } from '$lib/core/nav.svelte'
  import { formatRelativeAge } from '$lib/format/relative-time'
  import { workspaceStatusLine } from '$lib/state/activity'
  import { appStatusView } from '$lib/run-app/status'
  import { statusDotFor } from '$lib/sidebar/status'
  import { nav } from '$lib/core/nav.svelte'
  import type { ThreadListRowItem } from '$lib/sidebar/thread-list-store.svelte'
  import { cn } from '$lib/utils'
  import LoaderCircle from '@lucide/svelte/icons/loader-circle'
  import AlarmClock from '@lucide/svelte/icons/alarm-clock'
  import AlarmClockOff from '@lucide/svelte/icons/alarm-clock-off'
  import * as ContextMenu from '$lib/components/ui/context-menu'
  import { SNOOZE_OPTIONS, formatSnoozeUntil } from '$lib/sidebar/snooze'
  import { snoozes } from '$lib/sidebar/snooze-store.svelte'
  import { shellDialogs } from '$lib/state/shell-dialogs.svelte'
  import Trash2 from '@lucide/svelte/icons/trash-2'
  import StatusDot from './StatusDot.svelte'

  let { item, nowMs }: { item: ThreadListRowItem; nowMs: number } = $props()

  const workspace = $derived(item.workspace)
  const attention = $derived(item.urgency === 'attention')
  // In progress isn't your problem: a spinner, and everything else steps back.
  const working = $derived(workspace != null && !attention)

  const isCurrent = $derived(
    workspace
      ? nav.view === 'workspace' && nav.workspaceId === item.id
      : nav.view === 'scratch' && nav.scratchId === item.id,
  )

  const status = $derived(
    workspace
      ? workspaceStatusLine(workspace)
      : { text: 'Read-only', failed: false },
  )
  const statusLine = $derived(status.text)

  const dotVariant = $derived(
    workspace ? statusDotFor(workspace, attention) : 'idle',
  )

  // Only what the dot can't say: a setup that failed, or a leftover that can't be prompted.
  const showStatus = $derived(!workspace || status.failed)

  // The workspace's dev app, shown so a running server is findable without opening it.
  const appPort = $derived(workspace ? appStatusView(workspace).port : null)

  const age = $derived(
    item.activityAtMs ? formatRelativeAge(item.activityAtMs, nowMs) : null,
  )

  const snoozedUntil = $derived(item.snoozedUntilMs ?? null)

  // The snooze menu's times are worked out when it opens, so they match the moment you pick.
  let menuNowMs = $state(Date.now())

  const ariaLabel = $derived(
    [
      item.title,
      item.repoName,
      item.branch,
      attention ? `needs attention, ${statusLine}` : statusLine,
      appPort ? `app running on port ${appPort}` : null,
      item.parentTitle ? `started by ${item.parentTitle}` : null,
      snoozedUntil
        ? `snoozed until ${formatSnoozeUntil(snoozedUntil, nowMs)}`
        : null,
    ]
      .filter(Boolean)
      .join(', '),
  )

  // The same confirmation as the header's: a workspace's teardown, or a scratch's delete.
  function confirmDelete() {
    if (workspace) shellDialogs.open('teardown', { workspaceId: item.id })
    else shellDialogs.open('endScratch', { scratchId: item.id })
  }

  function open() {
    if (workspace) openWorkspace(item.id)
    else navigate({ view: 'scratch', scratchId: item.id })
  }
</script>

<ContextMenu.Root
  onOpenChange={(open) => {
    if (open) menuNowMs = Date.now()
  }}
>
  <ContextMenu.Trigger class="block">
    <button
      type="button"
      class={cn(
        'thread-row relative grid w-full grid-cols-[16px_minmax(0,1fr)] gap-x-2 overflow-hidden rounded-lg px-2.5 py-2 text-left transition-[background-color,transform,box-shadow] duration-(--dur-fast) ease-(--ease-out-snappy) outline-none hover:bg-muted focus-visible:ring-3 focus-visible:ring-ring/50 active:scale-[0.98] aria-[current=page]:bg-sidebar-accent dark:hover:bg-muted/50',
        item.nested && 'pl-6',
      )}
      aria-current={isCurrent ? 'page' : undefined}
      aria-label={ariaLabel}
      title={item.title}
      data-od-id="thread-row-{item.id}"
      data-urgency={item.urgency}
      onclick={open}
    >
      <span class="flex items-center">
        {#if working}
          <LoaderCircle
            class="size-3.5 animate-spin text-muted-foreground"
            data-od-id="thread-row-spinner"
          />
        {:else}
          <StatusDot variant={dotVariant} />
        {/if}
      </span>
      <span
        class={cn(
          'grid min-w-0 gap-0.5 transition-opacity duration-(--dur-med)',
          working && 'opacity-50',
        )}
        aria-hidden="true"
      >
        <span class="flex min-w-0 items-baseline gap-2">
          <span
            class={cn(
              'min-w-0 flex-1 truncate text-sm',
              working ? 'text-muted-foreground' : 'text-sidebar-foreground',
            )}>{item.title}</span
          >
          {#if snoozedUntil}
            <span
              class="flex shrink-0 items-center gap-1 font-mono text-[10px] text-muted-foreground/80 tabular-nums"
              data-od-id="thread-row-snoozed-until"
              title="Snoozed until {formatSnoozeUntil(snoozedUntil, nowMs)}"
              ><AlarmClock
                class="size-3"
                aria-hidden="true"
              />{formatSnoozeUntil(snoozedUntil, nowMs, { short: true })}</span
            >
          {:else if age}
            <span
              class="shrink-0 font-mono text-[10px] text-muted-foreground/80 tabular-nums"
              >{age}</span
            >
          {/if}
        </span>
        {#if item.repoName || item.branch}
          <span
            class="flex min-w-0 items-center gap-1 text-[11px] text-muted-foreground"
          >
            {#if item.repoName}
              <span class="max-w-[55%] shrink-0 truncate">{item.repoName}</span>
            {/if}
            {#if item.repoName && item.branch}
              <span class="opacity-60">·</span>
            {/if}
            {#if item.branch}
              <span class="min-w-0 truncate font-mono text-[10px]"
                >{item.branch}</span
              >
            {/if}
            {#if appPort}
              <span
                class="ml-auto flex shrink-0 items-center gap-1 font-mono text-[10px] text-emerald-600"
                title="App running on localhost:{appPort}"
              >
                <span class="size-1.5 rounded-full bg-emerald-500"
                ></span>:{appPort}
              </span>
            {/if}
          </span>
        {/if}
        {#if item.parentTitle && !item.nested}
          <span class="truncate text-[11px] text-muted-foreground">
            Started by {item.parentTitle}
          </span>
        {/if}
        {#if showStatus}
          <span
            class={cn(
              'flex min-w-0 items-center gap-1 text-xs text-muted-foreground',
            )}
          >
            <span class="truncate">{statusLine}</span>
          </span>
        {/if}
      </span>
    </button>
  </ContextMenu.Trigger>
  <ContextMenu.Content data-od-id="thread-row-menu">
    {#if snoozedUntil}
      <ContextMenu.Item onSelect={() => snoozes.unsnooze(item.id)}>
        <AlarmClockOff aria-hidden="true" />
        Unsnooze
      </ContextMenu.Item>
      <ContextMenu.Separator />
    {/if}
    <ContextMenu.Label
      >{snoozedUntil ? 'Snooze instead for' : 'Snooze for'}</ContextMenu.Label
    >
    {#each SNOOZE_OPTIONS as option (option.id)}
      <ContextMenu.Item
        data-od-id="snooze-{option.id}"
        onSelect={() => snoozes.snooze(item.id, option.until(Date.now()))}
      >
        <span class="flex-1">{option.label}</span>
        <span class="font-mono text-[10px] text-muted-foreground tabular-nums"
          >{formatSnoozeUntil(option.until(menuNowMs), menuNowMs)}</span
        >
      </ContextMenu.Item>
    {/each}
    <ContextMenu.Separator />
    <ContextMenu.Item
      class="text-destructive focus:bg-destructive/10 focus:text-destructive focus:**:text-destructive dark:focus:bg-destructive/20"
      data-od-id="thread-row-delete"
      onSelect={confirmDelete}
    >
      <Trash2 aria-hidden="true" />
      Delete…
    </ContextMenu.Item>
  </ContextMenu.Content>
</ContextMenu.Root>
