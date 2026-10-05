<script lang="ts">
  import { cn } from '$lib/utils'

  let { class: className }: { class?: string } = $props()
</script>

<!-- Three clay beads hop in turn. Reduce-motion stops the loop globally. -->
<span
  class={cn('agent-dots inline-flex items-end gap-[5px]', className)}
  aria-hidden="true"
>
  <span class="bead clay-roll size-3.5 bg-coral"></span>
  <span class="bead clay-roll size-3.5 bg-custard"></span>
  <span class="bead clay-roll size-3.5 bg-pond"></span>
</span>
