<script lang="ts">
  import { Toaster } from '$lib/components/ui/sonner'
</script>

<div
  id="toasts"
  class="toasts pointer-events-none fixed inset-0 z-[100]"
  role="status"
  aria-live="polite"
  aria-atomic="false"
  aria-relevant="additions"
>
  <Toaster
    position="bottom-right"
    duration={3600}
    closeButton={false}
    expand={true}
    visibleToasts={8}
    class="pointer-events-auto"
  />
</div>
