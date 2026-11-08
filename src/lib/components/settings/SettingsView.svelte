<script lang="ts">
  import { windowDrag } from '$lib/primitives/window-drag'
  import { tick } from 'svelte'
  import SettingsSectionHead from '$lib/components/settings/SettingsSectionHead.svelte'
  import SettingsSectionNav from '$lib/components/settings/SettingsSectionNav.svelte'
  import {
    SETTINGS_SECTIONS,
    SETTINGS_SECTION_IDS,
    type SettingsSectionId,
  } from '$lib/components/settings/sections'
  import { normalizeSettingsSection } from '$lib/settings/sections'
  import { app, settings } from '$lib/state'

  const SCROLL_SPY_OFFSET = 120

  let settingsTitle: HTMLHeadingElement | undefined = $state()
  let scrollRoot: HTMLElement | undefined = $state()
  let activeSection = $state<SettingsSectionId>(SETTINGS_SECTIONS[0].id)

  const sectionRefs: Partial<Record<SettingsSectionId, HTMLElement>> = {}

  function bindSection(node: HTMLElement, id: SettingsSectionId) {
    sectionRefs[id] = node
    return {
      destroy() {
        delete sectionRefs[id]
      },
    }
  }

  $effect(() => {
    if (app.focusTarget !== 'settings') return
    void app.focusGeneration
    settingsTitle?.focus()
  })

  $effect(() => {
    const raw = settings.focusSection
    if (!raw) return
    const section = normalizeSettingsSection(
      raw,
      SETTINGS_SECTION_IDS,
      'agents',
    )
    void tick().then(() => jumpTo(section))
    settings.focusSection = null
  })

  function jumpTo(section: SettingsSectionId) {
    const target = sectionRefs[section]
    if (!target) return
    target.scrollIntoView({
      behavior: settings.reduceMotion ? 'auto' : 'smooth',
      block: 'start',
    })
    target.focus({ preventScroll: true })
    activeSection = section
  }

  function updateActiveSection() {
    const root = scrollRoot
    if (!root) return
    const atBottom = root.scrollHeight - root.scrollTop - root.clientHeight < 8
    const last = SETTINGS_SECTIONS.at(-1)?.id ?? 'experimental'
    if (atBottom) {
      activeSection = last
      return
    }
    const rootTop = root.getBoundingClientRect().top
    let current: SettingsSectionId = SETTINGS_SECTIONS[0].id
    for (const section of SETTINGS_SECTIONS) {
      const el = sectionRefs[section.id]
      if (!el) continue
      const top = el.getBoundingClientRect().top - rootTop
      if (top <= SCROLL_SPY_OFFSET) current = section.id
    }
    activeSection = current
  }
</script>

<section
  class="view flex min-h-0 flex-1 flex-col"
  id="view-settings"
  data-od-id="settings-view"
>
  <div
    bind:this={scrollRoot}
    class="relative min-h-0 flex-1 overflow-y-auto"
    onscroll={updateActiveSection}
  >
    <div
      class="set-inner mx-auto flex w-full max-w-3xl flex-col gap-6 p-6 min-[761px]:max-w-4xl min-[761px]:flex-row min-[761px]:items-start min-[761px]:gap-10"
    >
      <SettingsSectionNav active={activeSection} onJump={jumpTo} />

      <div class="set-sections min-w-0 flex-1 space-y-10 pb-10">
        <!-- The title row doubles as the page's title bar: empty space in it drags the window. -->
        <header data-tauri-drag-region use:windowDrag>
          <h1
            bind:this={settingsTitle}
            tabindex="-1"
            class="text-xl font-semibold tracking-tight outline-none"
            data-od-id="settings-title"
          >
            Settings
          </h1>
        </header>

        {#each SETTINGS_SECTIONS as section (section.id)}
          {@const Panel = section.component}
          <section
            use:bindSection={section.id}
            id="settings-{section.id}"
            class="set-sec scroll-mt-6 outline-none"
            tabindex="-1"
            aria-labelledby="settings-{section.id}-h"
            data-od-id={'odId' in section
              ? section.odId
              : `settings-${section.id}`}
          >
            <SettingsSectionHead
              id="settings-{section.id}-h"
              title={section.label}
              description={section.description}
            />
            {#if 'flush' in section}
              <Panel />
            {:else}
              <div class="mt-4">
                <Panel />
              </div>
            {/if}
          </section>
        {/each}
      </div>
    </div>
  </div>
</section>
