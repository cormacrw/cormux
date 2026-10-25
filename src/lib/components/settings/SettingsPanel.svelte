<script lang="ts">
  import type { Snippet } from 'svelte'
  import { cn } from '$lib/utils'

  let {
    class: className,
    children,
    ...restProps
  }: { class?: string; children: Snippet } & Record<string, unknown> = $props()
</script>

<div class={cn('glass-card rounded-2xl', className)} {...restProps}>
  {@render children()}
</div>
