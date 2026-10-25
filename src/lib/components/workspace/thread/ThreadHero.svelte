<!--
  An empty conversation: a big greeting just above the prompt bar instead of
  the last conversation's messages, with cards to resume an earlier
  conversation or start on a task. Under the greeting: the agent, and the
  branch and workspace it works in. As wide as the prompt bar (760px, or the
  thread column less the bar's 4px sides), so the two line up. Its box runs
  down to the bar's top edge (past the bar's 8px top padding), so sending the
  first prompt drops it out of view right behind the bar.
-->
<script lang="ts">
  import { tick } from 'svelte'
  import * as Select from '$lib/components/ui/select'
  import { formatRelativeAge } from '$lib/homebase/relative-time'
  import { clickup } from '$lib/state/clickup.svelte'
  import { settings } from '$lib/state'
  import { composerDrafts } from '$lib/state/composer-drafts.svelte'
  import { todos } from '$lib/state/todos.svelte'
  import { weekPlan } from '$lib/state/week-plan.svelte'
  import { heroLine, type Conversation } from '$lib/thread/conversations'
  import { resumeConversation } from '$lib/thread/resume-conversation'
  import { taskPrompt } from '$lib/thread/task-prompt'
  import { weekItems } from '$lib/week/items'
  import Buddy from '$lib/components/buddy/Buddy.svelte'
  import { agentName } from '$lib/agent-name'
  import { buddyColor, buddyMoodForThread } from '$lib/buddy'
  import type { Thread } from '$lib/state/threads.svelte'
  import type { Workspace } from '$lib/state/workspaces.svelte'
  import GitBranch from '@lucide/svelte/icons/git-branch'
  import History from '@lucide/svelte/icons/history'
  import ListTodo from '@lucide/svelte/icons/list-todo'

  let {
    thread,
    workspace,
    conversationId,
    earlier,
    nowMs,
    onFocusComposer,
  }: {
    thread: Thread
    workspace: Workspace
    conversationId: string
    earlier: Conversation[]
    nowMs: number
    onFocusComposer: () => void
  } = $props()

  const threadId = $derived(thread.id)
  const line = $derived(heroLine(`${threadId}:${conversationId}`))

  let resuming = $state(false)

  // Open TODOs and sprint tasks, as on the Week page.
  const tasks = $derived(
    weekItems(todos.items, clickup.mine, weekPlan.rows).filter(
      (item) => !item.done,
    ),
  )

  function title(conversation: Conversation) {
    return conversation.firstPrompt ?? 'Untitled conversation'
  }

  function meta(conversation: Conversation) {
    const count = `${conversation.prompts} ${conversation.prompts === 1 ? 'message' : 'messages'}`
    return `${formatRelativeAge(conversation.lastAtMs, nowMs)} · ${count}`
  }

  // Picking one resumes it straight away, like picking a task types its prompt.
  async function resume(id: string) {
    const conversation = earlier.find((row) => row.id === id)
    if (!conversation || resuming) return
    resuming = true
    try {
      await resumeConversation(threadId, conversation)
    } finally {
      resuming = false
    }
  }

  // Types a prompt for the task; you send it when it reads right.
  async function pickTask(key: string) {
    const item = tasks.find((task) => task.key === key)
    if (!item) return
    composerDrafts.setFor(threadId, taskPrompt(item, settings.taskPrompt))
    await tick()
    onFocusComposer()
  }
</script>

<div
  data-conversation-part
  class="thread-hero mt-auto -mb-2 flex w-[min(760px,calc(100cqw-8px))] flex-col gap-5 self-center pt-10 pb-10"
  data-od-id="thread-hero"
>
  <h2
    class="text-4xl font-semibold tracking-tight text-balance text-foreground sm:text-5xl"
  >
    {line}
  </h2>

  <!-- Who you're talking to, and where they'll work. -->
  <p
    class="hero-who -mt-2 flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1 text-sm text-muted-foreground"
    data-od-id="thread-hero-who"
  >
    <span class="flex items-center gap-1.5 font-medium text-foreground">
      <Buddy
        mood={buddyMoodForThread(thread)}
        color={buddyColor(thread.id)}
        glasses={thread.role === 'Reviewer'}
        size={20}
      />
      {agentName(thread.id)}
    </span>
    <span aria-hidden="true">·</span>
    <span class="flex min-w-0 items-center gap-1 font-mono text-xs">
      <GitBranch class="size-3.5 shrink-0" aria-hidden="true" />
      <span class="truncate">{workspace.branch}</span>
    </span>
    <span aria-hidden="true">·</span>
    <span class="truncate">{workspace.name}</span>
  </p>

  {#if earlier.length || tasks.length}
    <div class="grid gap-3 sm:grid-cols-2">
      {#if earlier.length}
        <section
          class="glass-card flex flex-col gap-2 rounded-[16px] p-3"
          aria-labelledby="resume-title"
          data-od-id="resume-card"
        >
          <h3
            id="resume-title"
            class="flex items-center gap-1.5 px-2.5 text-xs font-medium text-muted-foreground"
          >
            <History class="size-3.5" aria-hidden="true" />
            Resume a conversation
          </h3>
          <Select.Root
            type="single"
            value=""
            onValueChange={(next) => void resume(next)}
          >
            <Select.Trigger
              class="w-full"
              aria-label="Earlier conversation"
              disabled={resuming}
              data-od-id="resume-select"
            >
              <span class="truncate text-muted-foreground"
                >Pick a conversation…</span
              >
            </Select.Trigger>
            <Select.Content align="start" class="max-w-[min(400px,100vw-2rem)]">
              {#each earlier as conversation (conversation.id)}
                <Select.Item
                  value={conversation.id}
                  label={title(conversation)}
                  class="rounded-lg py-2 pl-2.5"
                >
                  <span class="flex min-w-0 flex-col gap-0.5">
                    <span class="truncate">{title(conversation)}</span>
                    <span class="truncate text-xs text-muted-foreground"
                      >{meta(conversation)}</span
                    >
                  </span>
                </Select.Item>
              {/each}
            </Select.Content>
          </Select.Root>
        </section>
      {/if}

      {#if tasks.length}
        <section
          class="glass-card flex flex-col gap-2 rounded-[16px] p-3"
          aria-labelledby="task-title"
          data-od-id="task-card"
        >
          <h3
            id="task-title"
            class="flex items-center gap-1.5 px-2.5 text-xs font-medium text-muted-foreground"
          >
            <ListTodo class="size-3.5" aria-hidden="true" />
            Work on a task
          </h3>
          <Select.Root type="single" value="" onValueChange={pickTask}>
            <Select.Trigger
              class="w-full"
              aria-label="Task to work on"
              data-od-id="task-select"
            >
              <span class="truncate text-muted-foreground">Pick a task…</span>
            </Select.Trigger>
            <Select.Content align="start" class="max-w-[min(400px,100vw-2rem)]">
              {#each tasks as task (task.key)}
                <Select.Item
                  value={task.key}
                  label={task.title}
                  class="rounded-lg py-2 pl-2.5"
                >
                  <span class="flex min-w-0 items-center gap-2">
                    <span
                      class="shrink-0 rounded bg-foreground/10 px-1 font-mono text-[10px] text-muted-foreground"
                      >{task.tag}</span
                    >
                    <span class="truncate">{task.title}</span>
                  </span>
                </Select.Item>
              {/each}
            </Select.Content>
          </Select.Root>
        </section>
      {/if}
    </div>
  {/if}
</div>

<style>
  /* The greeting rises in, then each card just after it. */
  .thread-hero h2,
  .hero-who,
  .thread-hero section {
    animation: hero-in var(--dur-slide) var(--ease-out-snappy) backwards;
  }

  .hero-who {
    animation-delay: 40ms;
  }

  .thread-hero section {
    animation-delay: 90ms;
  }

  .thread-hero section + section {
    animation-delay: 140ms;
  }

  @keyframes hero-in {
    from {
      opacity: 0;
      transform: translate3d(0, 10px, 0);
    }
  }
</style>
