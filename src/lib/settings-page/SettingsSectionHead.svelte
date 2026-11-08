<script lang="ts">
  let {
    id,
    title,
    description,
  }: { id: string; title: string; description: string } = $props()
</script>

<div class="space-y-1">
  <h2 {id} class="text-lg font-semibold tracking-tight">{title}</h2>
  <p class="text-sm text-muted-foreground">{description}</p>
</div>
