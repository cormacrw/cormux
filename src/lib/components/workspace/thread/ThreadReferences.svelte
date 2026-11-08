<!--
  @component
  The PRs, tickets and docs a thread's agent pinned with `attach_entity`, beside the conversation.
-->
<script lang="ts">
  import { fly } from 'svelte/transition'
  import { openUrl } from '@tauri-apps/plugin-opener'
  import type { Component } from 'svelte'
  import type { EntityRow } from '$lib/ipc/bindings'
  import { motionMs } from '$lib/motion'
  import { entityLook, type EntityKind } from '$lib/thread/entities'
  import CircleDot from '@lucide/svelte/icons/circle-dot'
  import FileText from '@lucide/svelte/icons/file-text'
  import GitPullRequest from '@lucide/svelte/icons/git-pull-request'
  import Link from '@lucide/svelte/icons/link'
  import Ticket from '@lucide/svelte/icons/ticket'

  let { entities }: { entities: EntityRow[] } = $props()

  const ICONS: Record<EntityKind, Component> = {
    pullRequest: GitPullRequest,
    issue: CircleDot,
    ticket: Ticket,
    doc: FileText,
    link: Link,
  }

  function open(event: MouseEvent, url: string) {
    event.preventDefault()
    void openUrl(url).catch(() => {})
  }
</script>

<aside
  class="thread-references glass-pane flex max-h-full min-h-0 flex-col overflow-hidden rounded-[18px]"
  aria-labelledby="thread-references-heading"
  data-od-id="thread-references"
>
  <h2
    id="thread-references-heading"
    class="flex items-center justify-between px-3.5 pt-3 pb-1.5 text-[11px] font-medium tracking-wide text-muted-foreground uppercase"
  >
    References
    <span class="tabular-nums">{entities.length}</span>
  </h2>
  <ul class="min-h-0 overflow-y-auto px-1.5 pb-1.5">
    {#each entities as entity (entity.id)}
      {@const look = entityLook(entity.url)}
      {@const Icon = ICONS[look.kind]}
      <li in:fly={{ y: 4, duration: motionMs(180) }}>
        <a
          href={entity.url}
          title={entity.url}
          class="group flex items-start gap-2.5 rounded-[10px] px-2 py-1.5 hover:bg-foreground/5 focus-visible:bg-foreground/5 focus-visible:outline-none"
          onclick={(event) => open(event, entity.url)}
        >
          <Icon
            class="mt-0.5 size-3.5 shrink-0 text-muted-foreground group-hover:text-foreground"
            aria-hidden="true"
          />
          <span class="min-w-0">
            <span
              class="line-clamp-2 text-[13px] leading-snug font-medium text-foreground/90"
              >{entity.name}</span
            >
            <span class="block truncate text-[11px] text-muted-foreground"
              >{look.source}</span
            >
          </span>
        </a>
      </li>
    {/each}
  </ul>
</aside>
