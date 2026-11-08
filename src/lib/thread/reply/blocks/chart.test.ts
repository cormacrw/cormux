import { describe, expect, it } from 'vitest'
import { parseReply } from '../parse'
import { chartTicks } from './chart'

/** The chart a reply's fenced `chart` block draws, or null when it stays code. */
function chartIn(fields: object) {
  const spec = JSON.stringify({
    type: 'bar',
    labels: ['a', 'b'],
    series: [{ name: 'x', values: [1, 2] }],
    ...fields,
  })
  const [segment] = parseReply(`\`\`\`chart\n${spec}\n\`\`\``, {
    streaming: false,
  })
  return segment?.kind === 'block' && segment.block.kind === 'chart'
    ? segment.block.data
    : null
}

describe('a chart block', () => {
  it('draws the chart the agent described', () => {
    expect(chartIn({ title: 'Size' })).toEqual({
      type: 'bar',
      title: 'Size',
      labels: ['a', 'b'],
      series: [{ name: 'x', values: [1, 2] }],
    })
  })

  it('stays code when it can’t be drawn honestly', () => {
    expect(chartIn({ type: 'pie' })).toBeNull()
    expect(chartIn({ series: [{ name: 'x', values: [1] }] })).toBeNull()
    expect(chartIn({ series: [{ name: 'x', values: [1, 'two'] }] })).toBeNull()
    expect(chartIn({ labels: [] })).toBeNull()
  })

  it('draws no more series than the palette has colours', () => {
    const series = Array.from({ length: 7 }, (_, at) => ({
      name: `s${at}`,
      values: [1, 2],
    }))
    expect(chartIn({ series })?.series).toHaveLength(5)
  })
})

describe('chartTicks', () => {
  it('lands on round numbers past the top value', () => {
    expect(chartTicks(87)).toEqual([0, 25, 50, 75, 100])
    expect(chartTicks(0)).toEqual([0, 1])
  })
})
