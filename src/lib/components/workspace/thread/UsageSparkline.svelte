<script lang="ts">
  let { points }: { points: number[] } = $props()

  const WIDTH = 36
  const HEIGHT = 12

  const line = $derived.by(() => {
    const top = Math.max(...points, 1)
    const step = WIDTH / (points.length - 1)
    return points
      .map(
        (value, index) =>
          `${(index * step).toFixed(1)},${(HEIGHT - 1 - (value / top) * (HEIGHT - 2)).toFixed(1)}`,
      )
      .join(' ')
  })
</script>

<svg
  viewBox="0 0 {WIDTH} {HEIGHT}"
  width={WIDTH}
  height={HEIGHT}
  class="shrink-0 text-muted-foreground/70"
  aria-hidden="true"
  data-od-id="composer-usage-trend"
>
  <polyline
    points={line}
    fill="none"
    stroke="currentColor"
    stroke-width="1.25"
    stroke-linejoin="round"
    stroke-linecap="round"
  />
</svg>
