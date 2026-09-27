<script lang="ts">
  import { settings } from '$lib/state'
</script>

<section class="flex flex-1 flex-col gap-4 p-6">
  <header>
    <h1 class="text-xl font-semibold tracking-tight">Settings</h1>
  </header>
  <label class="flex items-center gap-2 text-sm">
    <input type="checkbox" bind:checked={settings.reduceMotion} />
    Reduce motion
  </label>
</section>
