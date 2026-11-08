import { describe, expect, it } from 'vitest'
import {
  bangName,
  expandBangs,
  filterBangs,
  paletteBang,
  type Bang,
} from './bangs'

const test: Bang = {
  id: '1',
  name: 'test',
  prompt: 'Run the tests and fix what fails.',
}
const review: Bang = { id: '2', name: 'review', prompt: 'Review the diff.' }
const bangs = [test, review, { id: '3', name: 'empty', prompt: ' ' }]

describe('bangName', () => {
  it('drops the !, spaces become dashes, lowercase, no symbols', () => {
    expect(bangName('!Daily Standup')).toBe('daily-standup')
    expect(bangName('  fix/it!  ')).toBe('fixit')
  })
})

describe('expandBangs', () => {
  it('expands whole known names only', () => {
    expect(expandBangs('!test then !nope and a!test', bangs)).toBe(
      'Run the tests and fix what fails. then !nope and a!test',
    )
    expect(expandBangs('!empty', bangs)).toBe('!empty')
  })
})

describe('filterBangs', () => {
  it('puts prefix matches first and skips unusable bangs', () => {
    expect(filterBangs(bangs, 'e').map((bang) => bang.name)).toEqual([
      'test',
      'review',
    ])
    expect(filterBangs(bangs, 're').map((bang) => bang.name)).toEqual([
      'review',
    ])
  })
})

describe('paletteBang', () => {
  it('reads a bang and the words after it', () => {
    expect(paletteBang('!test only the api', bangs)).toEqual({
      bang: test,
      extra: 'only the api',
    })
    expect(paletteBang('!review', bangs)).toEqual({ bang: review, extra: '' })
    expect(paletteBang('!tes', bangs)).toBeNull()
    expect(paletteBang('test', bangs)).toBeNull()
  })
})
