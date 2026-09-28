<script lang="ts">
  import {
    SETTINGS_SECTIONS,
    type SettingsSectionId,
    settingsSectionDomId,
  } from '$lib/settings/sections'
  import { cn } from '$lib/utils'

  let {
    active,
    onJump,
  }: {
    active: SettingsSectionId
    onJump: (section: SettingsSectionId) => void
  } = $props()
</script>

<nav
  class="set-nav flex gap-1 max-[760px]:flex-wrap min-[761px]:sticky min-[761px]:top-6 min-[761px]:max-h-[calc(100dvh-6rem)] min-[761px]:w-40 min-[761px]:shrink-0 min-[761px]:flex-col"
  aria-label="Settings sections"
  data-od-id="settings-nav"
>
  {#each SETTINGS_SECTIONS as section (section.id)}
    <button
      type="button"
      class={cn(
        'rounded-md px-3 py-1.5 text-left text-sm transition-colors',
        active === section.id
          ? 'bg-muted font-medium text-foreground'
          : 'text-muted-foreground hover:bg-muted/60 hover:text-foreground',
      )}
      aria-current={active === section.id ? 'true' : undefined}
      onclick={() => onJump(section.id)}
    >
      {section.label}
    </button>
  {/each}
</nav>

<!-- keep dom ids referenced for a11y tests -->
<span class="sr-only">{settingsSectionDomId(active)}</span>
