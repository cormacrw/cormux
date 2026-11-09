<script lang="ts">
  import JsonTree from './JsonTree.svelte'

  let { data }: { data: { value: unknown } } = $props()
</script>

<div class="reply-card reply-json" data-od-id="reply-json">
  <JsonTree value={data.value} />
</div>
