<!-- One row of the conversation, keyed for find; `pop` grows it in when it's new. -->
<script lang="ts">
  import type { Snippet } from 'svelte'
  import type { ClassValue } from 'svelte/elements'
  import type { RowContext } from './context'
  import { popIn } from './pop-in'

  let {
    id,
    ctx,
    pop = false,
    origin,
    class: className,
    children,
  }: {
    id: string
    ctx: RowContext
    /** False for rows that animate some other way (a tool run's steps pop, a reply types in). */
    pop?: boolean
    origin?: string
    class?: ClassValue
    children: Snippet
  } = $props()
</script>

<li
  use:ctx.trackRow={id}
  class={['py-1', className]}
  use:popIn={{ entries: ctx.entries, key: pop ? id : undefined, origin }}
>
  {@render children()}
</li>
