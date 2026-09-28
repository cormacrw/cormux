<script lang="ts">
  import { untrack } from 'svelte'
  import { Button } from '$lib/components/ui/button'
  import * as Dialog from '$lib/components/ui/dialog'
  import { Input } from '$lib/components/ui/input'
  import { commands } from '$lib/ipc'
  import { toastCoreError } from '$lib/feedback/wire-feedback'
  import { workspaces } from '$lib/state'

  let {
    workspaceId = $bindable(null as string | null),
  }: {
    workspaceId?: string | null
  } = $props()

  let name = $state('')
  let saving = $state(false)

  const open = $derived(workspaceId != null)

  // Seed once per opened workspace; untracked so snapshot refreshes don't wipe typing.
  $effect(() => {
    const id = workspaceId
    if (!id) return
    name = untrack(() => workspaces.getById(id)?.name ?? '')
  })

  function close() {
    workspaceId = null
  }

  async function save() {
    if (!workspaceId) return
    const trimmed = name.trim()
    if (!trimmed) return
    saving = true
    const result = await commands.renameWorkspace({
      workspaceId,
      name: trimmed,
    })
    saving = false
    if (result.status === 'error') {
      toastCoreError(result.error)
      return
    }
    workspaces.patchName(workspaceId, trimmed)
    close()
  }
</script>

<Dialog.Root
  {open}
  onOpenChange={(next) => {
    if (!next) close()
  }}
>
  <Dialog.Content class="sm:max-w-sm">
    <Dialog.Header>
      <Dialog.Title>Rename workspace</Dialog.Title>
      <Dialog.Description>
        Shown in the sidebar and on Homebase.
      </Dialog.Description>
    </Dialog.Header>
    <form
      class="grid gap-3"
      onsubmit={(event) => {
        event.preventDefault()
        void save()
      }}
    >
      <Input bind:value={name} aria-label="Workspace name" autofocus />
      <Dialog.Footer>
        <Button type="button" variant="ghost" onclick={close}>Cancel</Button>
        <Button type="submit" disabled={saving || !name.trim()}>
          {saving ? 'Saving…' : 'Save'}
        </Button>
      </Dialog.Footer>
    </form>
  </Dialog.Content>
</Dialog.Root>
