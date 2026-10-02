<script lang="ts">
  import { tick } from 'svelte'
  import AgentsSettings from '$lib/components/settings/AgentsSettings.svelte'
  import ClickupSettings from '$lib/components/settings/ClickupSettings.svelte'
  import AppearanceSettings from '$lib/components/settings/AppearanceSettings.svelte'
  import GeneralSettings from '$lib/components/settings/GeneralSettings.svelte'
  import GithubSettings from '$lib/components/settings/GithubSettings.svelte'
  import NotificationsSettings from '$lib/components/settings/NotificationsSettings.svelte'
  import PrPromptSettings from '$lib/components/settings/PrPromptSettings.svelte'
  import ReposSettings from '$lib/components/settings/ReposSettings.svelte'
  import ScratchMacrosSettings from '$lib/components/settings/ScratchMacrosSettings.svelte'
  import ShortcutsSettings from '$lib/components/settings/ShortcutsSettings.svelte'
  import SettingsSectionHead from '$lib/components/settings/SettingsSectionHead.svelte'
  import SettingsSectionNav from '$lib/components/settings/SettingsSectionNav.svelte'
  import {
    normalizeSettingsSection,
    SETTINGS_SECTIONS,
    type SettingsSectionId,
    settingsSectionDomId,
  } from '$lib/settings/sections'
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
    const section = normalizeSettingsSection(raw)
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
    const last = SETTINGS_SECTIONS.at(-1)?.id ?? 'shortcuts'
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
        <header>
          <h1
            bind:this={settingsTitle}
            tabindex="-1"
            class="text-xl font-semibold tracking-tight outline-none"
            data-od-id="settings-title"
          >
            Settings
          </h1>
        </header>

        <section
          use:bindSection={'general'}
          id={settingsSectionDomId('general')}
          class="set-sec scroll-mt-6 outline-none"
          tabindex="-1"
          aria-labelledby="settings-general-h"
          data-od-id="settings-general"
        >
          <SettingsSectionHead
            id="settings-general-h"
            title="General"
            description="Defaults for new workspaces and how the app behaves."
          />
          <div class="mt-4">
            <GeneralSettings />
          </div>
        </section>

        <section
          use:bindSection={'agents'}
          id={settingsSectionDomId('agents')}
          class="set-sec scroll-mt-6 outline-none"
          tabindex="-1"
          aria-labelledby="settings-agents-h"
          data-od-id="settings-agents"
        >
          <SettingsSectionHead
            id="settings-agents-h"
            title="Agents"
            description="Which engine new workspaces use, and whether they ask first."
          />
          <div class="mt-4">
            <AgentsSettings />
          </div>
        </section>

        <section
          use:bindSection={'github'}
          id={settingsSectionDomId('github')}
          class="set-sec scroll-mt-6 outline-none"
          tabindex="-1"
          aria-labelledby="settings-github-h"
          data-od-id="settings-github-section"
        >
          <SettingsSectionHead
            id="settings-github-h"
            title="GitHub"
            description="Pull requests sync through the gh CLI, and how Create PR drafts them."
          />
          <div class="mt-4">
            <GithubSettings />
            <PrPromptSettings />
          </div>
        </section>

        <section
          use:bindSection={'clickup'}
          id={settingsSectionDomId('clickup')}
          class="set-sec scroll-mt-6 outline-none"
          tabindex="-1"
          aria-labelledby="settings-clickup-h"
          data-od-id="settings-clickup-section"
        >
          <SettingsSectionHead
            id="settings-clickup-h"
            title="ClickUp"
            description="Your team's current sprint as a board."
          />
          <div class="mt-4">
            <ClickupSettings />
          </div>
        </section>

        <section
          use:bindSection={'repos'}
          id={settingsSectionDomId('repos')}
          class="set-sec scroll-mt-6 outline-none"
          tabindex="-1"
          aria-labelledby="settings-repos-h"
          data-od-id="settings-repos"
        >
          <SettingsSectionHead
            id="settings-repos-h"
            title="Repos"
            description="Local folders that new workspaces can check out from."
          />
          <ReposSettings />
        </section>

        <section
          use:bindSection={'macros'}
          id={settingsSectionDomId('macros')}
          class="set-sec scroll-mt-6 outline-none"
          tabindex="-1"
          aria-labelledby="settings-macros-h"
          data-od-id="settings-macros"
        >
          <SettingsSectionHead
            id="settings-macros-h"
            title="Scratch macros"
            description="Saved prompts you can start as a scratch from the command palette."
          />
          <ScratchMacrosSettings />
        </section>

        <section
          use:bindSection={'appearance'}
          id={settingsSectionDomId('appearance')}
          class="set-sec scroll-mt-6 outline-none"
          tabindex="-1"
          aria-labelledby="settings-appearance-h"
          data-od-id="settings-appearance"
        >
          <SettingsSectionHead
            id="settings-appearance-h"
            title="Appearance"
            description="Theme, primary color and text sizes on this Mac."
          />
          <div class="mt-4">
            <AppearanceSettings />
          </div>
        </section>

        <section
          use:bindSection={'notifications'}
          id={settingsSectionDomId('notifications')}
          class="set-sec scroll-mt-6 outline-none"
          tabindex="-1"
          aria-labelledby="settings-notifications-h"
          data-od-id="settings-notifications-section"
        >
          <SettingsSectionHead
            id="settings-notifications-h"
            title="Notifications"
            description="Desktop alerts while Cormux is in the background."
          />
          <div class="mt-4">
            <NotificationsSettings />
          </div>
        </section>

        <section
          use:bindSection={'shortcuts'}
          id={settingsSectionDomId('shortcuts')}
          class="set-sec scroll-mt-6 outline-none"
          tabindex="-1"
          aria-labelledby="settings-shortcuts-h"
          data-od-id="settings-shortcuts"
        >
          <SettingsSectionHead
            id="settings-shortcuts-h"
            title="Keyboard shortcuts"
            description="Keys for getting around Cormux."
          />
          <div class="mt-4">
            <ShortcutsSettings />
          </div>
        </section>
      </div>
    </div>
  </div>
</section>
