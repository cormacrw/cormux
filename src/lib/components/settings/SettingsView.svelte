<script lang="ts">
  import { app, settings } from '$lib/state'

  let settingsTitle: HTMLHeadingElement | undefined = $state()

  $effect(() => {
    if (app.focusTarget !== 'settings') return
    void app.focusGeneration
    settingsTitle?.focus()
  })
</script>

<section class="flex flex-1 flex-col gap-4 p-6">
  <header>
    <h1
      bind:this={settingsTitle}
      tabindex="-1"
      class="text-xl font-semibold tracking-tight outline-none"
    >
      Settings
    </h1>
  </header>
  <label class="flex items-center gap-2 text-sm">
    <input type="checkbox" bind:checked={settings.reduceMotion} />
    Reduce motion
  </label>
</section>
