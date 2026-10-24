<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { workspaceUi } from '$lib/state'
  import X from '@lucide/svelte/icons/x'

  let { label }: { label: string } = $props()
</script>

<Button
  variant="secondary"
  size="icon-sm"
  class="shrink-0 rounded-full"
  aria-label={label}
  title="{label} (Esc)"
  data-od-id="slide-out-close"
  onclick={() => workspaceUi.closeSlideOut()}
>
  <X class="size-3.5" aria-hidden="true" />
</Button>
