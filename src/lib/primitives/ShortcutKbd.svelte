<script lang="ts">
  import CommandIcon from '@lucide/svelte/icons/command'
  import { Kbd } from '$lib/components/ui/kbd'
  import { SHORTCUTS, type ShortcutId } from '$lib/keyboard/shortcuts'

  type LetterShortcut = {
    [K in ShortcutId]: (typeof SHORTCUTS)[K] extends { key: string } ? K : never
  }[ShortcutId]

  let {
    id,
    class: className = 'gap-0.5',
    iconClass,
  }: { id: LetterShortcut; class?: string; iconClass?: string } = $props()
</script>

<Kbd class={className} aria-hidden="true"
  ><CommandIcon class={iconClass} />{SHORTCUTS[id].key.toUpperCase()}</Kbd
>
