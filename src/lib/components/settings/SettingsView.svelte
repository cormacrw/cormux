<script lang="ts">
  import ReposSettings from '$lib/components/settings/ReposSettings.svelte'
  import { app, settings } from '$lib/state'

  let settingsTitle: HTMLHeadingElement | undefined = $state()

  let reposSection: HTMLElement | undefined = $state()
  let enginesSection: HTMLElement | undefined = $state()

  $effect(() => {
    if (app.focusTarget !== 'settings') return
    void app.focusGeneration
    settingsTitle?.focus()
  })

  $effect(() => {
    const section = settings.focusSection
    if (!section) return
    const target =
      section === 'repos'
        ? reposSection
        : section === 'engines'
          ? enginesSection
          : null
    target?.scrollIntoView({ block: 'nearest' })
    settings.focusSection = null
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
  <section bind:this={reposSection} id="settings-repos" class="scroll-mt-4">
    <h2 class="text-sm font-medium">Repositories</h2>
    <p class="text-sm text-muted-foreground">Repo setup and run commands.</p>
    <ReposSettings />
  </section>
  <section bind:this={enginesSection} id="settings-engines" class="scroll-mt-4">
    <h2 class="text-sm font-medium">Engines</h2>
    <p class="text-sm text-muted-foreground">
      Default agent and installed CLIs.
    </p>
  </section>
</section>
