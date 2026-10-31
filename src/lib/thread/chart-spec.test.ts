import { describe, expect, it } from 'vitest'
import { chartTicks, MAX_SERIES, parseChartSpec } from './chart-spec'

const spec = (fields: object) =>
  JSON.stringify({
    type: 'bar',
    labels: ['a', 'b'],
    series: [{ name: 'x', values: [1, 2] }],
    ...fields,
  })

describe('parseChartSpec', () => {
  it('reads a chart', () => {
    expect(parseChartSpec(spec({ title: 'Size' }))).toEqual({
      type: 'bar',
      title: 'Size',
      labels: ['a', 'b'],
      series: [{ name: 'x', values: [1, 2] }],
    })
  })

  it('refuses what it can’t draw honestly', () => {
    expect(parseChartSpec('nope')).toBeNull()
    expect(parseChartSpec(spec({ type: 'pie' }))).toBeNull()
    expect(
      parseChartSpec(spec({ series: [{ name: 'x', values: [1] }] })),
    ).toBeNull()
    expect(
      parseChartSpec(spec({ series: [{ name: 'x', values: [1, 'two'] }] })),
    ).toBeNull()
  })

  it('stops at the palette’s length', () => {
    const series = Array.from({ length: 7 }, (_, at) => ({
      name: `s${at}`,
      values: [1, 2],
    }))
    expect(parseChartSpec(spec({ series }))?.series).toHaveLength(MAX_SERIES)
  })
})

describe('chartTicks', () => {
  it('lands on round numbers past the top value', () => {
    expect(chartTicks(87)).toEqual([0, 25, 50, 75, 100])
    expect(chartTicks(0)).toEqual([0, 1])
  })
})
