<script lang="ts">
  import * as Empty from '$lib/components/ui/empty'
  import { prs } from '$lib/state'
</script>

<section
  class="grid gap-4"
  aria-labelledby="open-prs-title"
  data-od-id="open-prs"
>
  <div class="flex flex-wrap items-end justify-between gap-3">
    <h2 id="open-prs-title" class="text-sm font-medium tracking-tight">
      Open pull requests
      <span class="font-mono text-muted-foreground">{prs.count}</span>
    </h2>
  </div>
  {#if prs.count === 0}
    <Empty.Root class="border border-dashed border-border/80">
      <Empty.Header>
        <Empty.Title>No open pull requests</Empty.Title>
        <Empty.Description>
          Pull requests you open, or that ask for your review, show up here.
        </Empty.Description>
      </Empty.Header>
    </Empty.Root>
  {:else}
    <ul class="divide-y rounded-xl border border-border text-sm">
      {#each prs.items as pr (pr.id)}
        <li class="px-4 py-3">
          <span class="font-medium">{pr.title}</span>
          <span class="text-muted-foreground"> #{pr.number}</span>
        </li>
      {/each}
    </ul>
  {/if}
</section>
