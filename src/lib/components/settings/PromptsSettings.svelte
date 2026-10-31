<script lang="ts">
  import { onMount } from 'svelte'
  import { commands } from '$lib/ipc'
  import { settings } from '$lib/state'
  import PromptSetting from './PromptSetting.svelte'

  let defaultPrPrompt = $state('')

  onMount(async () => {
    defaultPrPrompt = await commands.defaultPrPrompt()
  })
</script>

<div class="space-y-4">
  {#if defaultPrPrompt}
    <PromptSetting
      id="pr-prompt"
      label="PR prompt"
      hint="What the agent is asked when Create PR drafts a description. The thread, goal and changed files are added after it."
      saved={settings.prPrompt}
      defaultPrompt={defaultPrPrompt}
      onSave={(next) => void settings.setPrPrompt(next)}
    />
  {/if}
</div>
