<script lang="ts">
  import CommandIcon from '@lucide/svelte/icons/command'
  import CornerDownLeftIcon from '@lucide/svelte/icons/corner-down-left'
  import { Kbd } from '$lib/components/ui/kbd/index.js'

  // Sits inside a dialog's footer button: ⌘↵ on the action, Esc on Cancel.
  let { keys }: { keys: 'submit' | 'cancel' } = $props()
</script>

<Kbd
  class="ml-1 hidden gap-0.5 bg-current/12 px-1.5 text-current sm:inline-flex"
  aria-hidden="true"
>
  {#if keys === 'submit'}
    <CommandIcon />
    <CornerDownLeftIcon />
  {:else}
    Esc
  {/if}
</Kbd>
