<script lang="ts">
  import { chartTicks, type ChartSpec } from '$lib/thread/chart-spec'
  import { cn } from '$lib/utils'

  let { spec }: { spec: ChartSpec } = $props()

  const WIDTH = 640
  const HEIGHT = 240
  const PLOT = { top: 12, right: 12, bottom: 28, left: 44 }
  const plotWidth = WIDTH - PLOT.left - PLOT.right
  const plotHeight = HEIGHT - PLOT.top - PLOT.bottom
  const BAR_GAP = 2
  const MAX_BAR = 28

  const ticks = $derived(
    chartTicks(Math.max(...spec.series.flatMap((row) => row.values))),
  )
  const top = $derived(ticks.at(-1) ?? 1)
  const slot = $derived(plotWidth / spec.labels.length)
  const y = (value: number) =>
    PLOT.top + plotHeight * (1 - Math.max(value, 0) / top)
  const center = (index: number) => PLOT.left + slot * (index + 0.5)

  const barWidth = $derived(
    Math.min(
      MAX_BAR,
      (slot * 0.7 - BAR_GAP * (spec.series.length - 1)) / spec.series.length,
    ),
  )
  function barX(index: number, series: number) {
    const count = spec.series.length
    const group = barWidth * count + BAR_GAP * (count - 1)
    return center(index) - group / 2 + (barWidth + BAR_GAP) * series
  }

  /** A bar with rounded data-end, anchored square on the baseline. */
  function barPath(x: number, value: number) {
    const bottom = y(0)
    const height = bottom - y(value)
    const r = Math.min(4, barWidth / 2, height)
    return `M${x},${bottom}V${bottom - height + r}Q${x},${bottom - height} ${x + r},${bottom - height}H${x + barWidth - r}Q${x + barWidth},${bottom - height} ${x + barWidth},${bottom - height + r}V${bottom}Z`
  }

  let hovered = $state<number | null>(null)
  let showTable = $state(false)

  const hoverLeft = $derived(
    hovered === null ? 0 : (center(hovered) / WIDTH) * 100,
  )

  function format(value: number) {
    return value.toLocaleString()
  }
</script>

<figure class="reply-card reply-chart" data-od-id="reply-chart">
  <figcaption class="reply-card-head">
    {#if spec.title}
      <span class="truncate font-medium text-foreground">{spec.title}</span>
    {/if}
    <button
      type="button"
      class="ml-auto shrink-0 font-medium text-foreground/80 hover:text-foreground"
      aria-pressed={showTable}
      onclick={() => (showTable = !showTable)}
      >{showTable ? 'Chart' : 'Table'}</button
    >
  </figcaption>

  {#if showTable}
    <div class="agent-markdown px-3 pb-3">
      <table>
        <thead>
          <tr>
            <th></th>
            {#each spec.series as row (row.name)}<th align="right"
                >{row.name}</th
              >{/each}
          </tr>
        </thead>
        <tbody>
          {#each spec.labels as label, index (index)}
            <tr>
              <td>{label}</td>
              {#each spec.series as row (row.name)}
                <td align="right">{format(row.values[index] ?? 0)}</td>
              {/each}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {:else}
    {#if spec.series.length > 1}
      <ul class="reply-chart-legend">
        {#each spec.series as row, index (row.name)}
          <li>
            <span
              class="size-2 rounded-sm"
              style:background="var(--viz-{index + 1})"
            ></span>
            {row.name}
          </li>
        {/each}
      </ul>
    {/if}
    <div class="relative">
      <svg
        viewBox="0 0 {WIDTH} {HEIGHT}"
        class="block w-full"
        role="img"
        aria-label={spec.title ?? 'Chart'}
        onpointerleave={() => (hovered = null)}
      >
        {#each ticks as tick (tick)}
          <line
            x1={PLOT.left}
            x2={WIDTH - PLOT.right}
            y1={y(tick)}
            y2={y(tick)}
            class="stroke-border"
          />
          <text
            x={PLOT.left - 8}
            y={y(tick) + 3.5}
            text-anchor="end"
            class="fill-muted-foreground text-[10px]">{format(tick)}</text
          >
        {/each}
        {#each spec.labels as label, index (index)}
          {#if hovered === index}
            <rect
              x={PLOT.left + slot * index}
              y={PLOT.top}
              width={slot}
              height={plotHeight}
              class="fill-muted"
            />
          {/if}
          <text
            x={center(index)}
            y={HEIGHT - 10}
            text-anchor="middle"
            class="fill-muted-foreground text-[10px]">{label}</text
          >
        {/each}

        {#each spec.series as row, series (row.name)}
          {#if spec.type === 'bar'}
            {#each row.values as value, index (index)}
              <path
                d={barPath(barX(index, series), value)}
                fill="var(--viz-{series + 1})"
              />
            {/each}
          {:else}
            <polyline
              points={row.values
                .map((value, index) => `${center(index)},${y(value)}`)
                .join(' ')}
              fill="none"
              stroke="var(--viz-{series + 1})"
              stroke-width="2"
              stroke-linejoin="round"
              stroke-linecap="round"
            />
            {#if hovered !== null}
              <circle
                cx={center(hovered)}
                cy={y(row.values[hovered] ?? 0)}
                r="4"
                fill="var(--viz-{series + 1})"
                class="stroke-background-solid"
                stroke-width="2"
              />
            {/if}
          {/if}
        {/each}

        <!-- The hit area is the whole slot, bigger than any mark in it. -->
        {#each spec.labels as label, index (index)}
          <rect
            role="presentation"
            aria-label={label}
            x={PLOT.left + slot * index}
            y={PLOT.top}
            width={slot}
            height={plotHeight}
            fill="transparent"
            data-chart-slot={index}
            onpointerenter={() => (hovered = index)}
          />
        {/each}
      </svg>
      {#if hovered !== null}
        <div
          class={cn('reply-chart-tip', hoverLeft > 60 && '-translate-x-full')}
          style:left="{hoverLeft}%"
          role="status"
        >
          <p class="font-medium">{spec.labels[hovered]}</p>
          {#each spec.series as row, index (row.name)}
            <p class="flex items-center gap-1.5">
              <span
                class="size-2 rounded-sm"
                style:background="var(--viz-{index + 1})"
              ></span>
              {#if row.name}<span class="text-muted-foreground">{row.name}</span
                >{/if}
              <span class="ml-auto pl-3 font-mono"
                >{format(row.values[hovered] ?? 0)}</span
              >
            </p>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</figure>
