<script lang="ts">
  import { onMount } from 'svelte'
  import { commands } from '$lib/ipc'
  import { settings } from '$lib/state'
  import { DEFAULT_TASK_PROMPT } from '$lib/thread/task-prompt'
  import PromptSetting from './PromptSetting.svelte'

  let defaultPrPrompt = $state('')

  onMount(async () => {
    defaultPrPrompt = await commands.defaultPrPrompt()
  })
</script>

<div class="space-y-4">
  <PromptSetting
    id="task-prompt"
    label="Task prompt"
    hint="Typed into the prompt bar when you pick a task to work on in an empty conversation. {'{task}'} becomes the task's #ID: a sprint task's ClickUp ID, or a TODO's CMUX-<n>. Sending adds its name and where the agent can read the rest."
    saved={settings.taskPrompt}
    defaultPrompt={DEFAULT_TASK_PROMPT}
    onSave={(next) => void settings.setTaskPrompt(next)}
  />
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
