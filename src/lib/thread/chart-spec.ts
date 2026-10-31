// A chart an agent writes as a fenced `chart` block of JSON.

export type ChartSeries = { name: string; values: number[] }

export type ChartSpec = {
  type: 'bar' | 'line'
  title?: string
  labels: string[]
  series: ChartSeries[]
}

/** The chart palette's length. Past it hues would repeat, so the rest aren't drawn. */
export const MAX_SERIES = 5

/** Null for anything that can't be drawn honestly; the block then stays code. */
export function parseChartSpec(source: string): ChartSpec | null {
  let raw: unknown
  try {
    raw = JSON.parse(source)
  } catch {
    return null
  }
  if (typeof raw !== 'object' || raw === null) return null
  const { type, title, labels, series } = raw as Record<string, unknown>
  if (type !== 'bar' && type !== 'line') return null
  if (!Array.isArray(labels) || !labels.length || !Array.isArray(series)) {
    return null
  }
  const names = labels.map(String)
  const drawn: ChartSeries[] = []
  for (const entry of series.slice(0, MAX_SERIES)) {
    const { name, values } = (entry ?? {}) as Record<string, unknown>
    if (!Array.isArray(values) || values.length !== names.length) return null
    if (!values.every((value) => Number.isFinite(value))) return null
    drawn.push({ name: String(name ?? ''), values: values as number[] })
  }
  if (!drawn.length) return null
  return {
    type,
    title: typeof title === 'string' ? title : undefined,
    labels: names,
    series: drawn,
  }
}

/** A round top for the axis, and the gridlines under it. Bars start at zero. */
export function chartTicks(max: number, count = 4): number[] {
  if (max <= 0) return [0, 1]
  const rough = max / count
  const power = 10 ** Math.floor(Math.log10(rough))
  const step = [1, 2, 2.5, 5, 10].find((nice) => nice * power >= rough)! * power
  const ticks = [0]
  while (ticks.at(-1)! < max) ticks.push(ticks.length * step)
  return ticks
}
