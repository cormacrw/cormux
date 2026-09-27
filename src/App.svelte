<script lang="ts">
  import Plus from '@lucide/svelte/icons/plus'
  import { Button } from '$lib/components/ui/button'
</script>

<main class="grid min-h-screen place-items-center">
  <div class="flex flex-col items-center gap-4">
    <h1 class="text-2xl font-semibold tracking-tight">Harness</h1>
    <Button>
      <Plus />
      New workspace
    </Button>
  </div>
</main>
