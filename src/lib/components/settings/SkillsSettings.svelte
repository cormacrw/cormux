<script lang="ts">
  import {
    Empty,
    EmptyDescription,
    EmptyHeader,
    EmptyTitle,
  } from '$lib/components/ui/empty'
</script>

<div
  class="rounded-lg border border-dashed border-border px-4 py-8"
  data-od-id="settings-skills-empty"
>
  <Empty>
    <EmptyHeader>
      <EmptyTitle>No skills yet</EmptyTitle>
      <EmptyDescription>
        Skills you add here will be available to every engine. Managing them
        from this page isn't built yet.
      </EmptyDescription>
    </EmptyHeader>
  </Empty>
</div>
