import { describe, expect, it } from 'vitest'
import {
  bangName,
  bangToken,
  expandBangs,
  filterBangs,
  insertBang,
  paletteBang,
  parseBangs,
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

describe('parseBangs', () => {
  it('reads the bangs row', () => {
    expect(parseBangs(JSON.stringify([test]))).toEqual([test])
  })

  it('carries scratch macros over until bangs are saved', () => {
    const macros = JSON.stringify([
      { id: 'm1', name: 'Daily standup', prompt: 'Summarise' },
    ])
    expect(parseBangs(undefined, macros)).toEqual([
      { id: 'm1', name: 'daily-standup', prompt: 'Summarise' },
    ])
    expect(parseBangs('[]', macros)).toEqual([])
  })

  it('drops anything malformed', () => {
    expect(parseBangs('nope')).toEqual([])
    expect(parseBangs(JSON.stringify([{ name: 'x' }, 3]))).toEqual([])
  })
})

describe('bangToken and insertBang', () => {
  it('finds the !word at the caret, at the start or after a space', () => {
    expect(bangToken('!te', 3)).toEqual({ start: 0, end: 3, query: 'te' })
    expect(bangToken('please !re now', 10)).toEqual({
      start: 7,
      end: 10,
      query: 're',
    })
    expect(bangToken('wow!te', 6)).toBeNull()
  })

  it('replaces the word with the prompt', () => {
    const token = bangToken('please !te', 10)!
    expect(insertBang('please !te', token, test)).toEqual({
      text: 'please Run the tests and fix what fails.',
      caret: 40,
    })
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
