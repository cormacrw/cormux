import { describe, expect, it } from 'vitest'
import { groupPrsByStack, prCards } from './pr-stacks'

const pr = (id: string, head: string, base: string, repo = 'o/r') => ({
  id,
  head,
  base,
  repoFullName: repo,
})

describe('groupPrsByStack', () => {
  it('leaves unrelated PRs as single rows in order', () => {
    const prs = [pr('1', 'a', 'main'), pr('2', 'b', 'main')]
    expect(groupPrsByStack(prs)).toEqual([
      { kind: 'single', pr: prs[0] },
      { kind: 'single', pr: prs[1] },
    ])
  })

  it('groups a chain top first, where its first PR was', () => {
    const solo = pr('0', 'solo', 'main')
    const top = pr('3', 'feat/c', 'feat/b')
    const bottom = pr('1', 'feat/a', 'main')
    const middle = pr('2', 'feat/b', 'feat/a')
    expect(groupPrsByStack([solo, top, bottom, middle])).toEqual([
      { kind: 'single', pr: solo },
      { kind: 'stack', id: '1', trunk: 'main', prs: [top, middle, bottom] },
    ])
  })

  it('keeps branches with the same name in different repos apart', () => {
    const prs = [pr('1', 'a', 'main', 'o/x'), pr('2', 'b', 'a', 'o/y')]
    expect(groupPrsByStack(prs).map((entry) => entry.kind)).toEqual([
      'single',
      'single',
    ])
  })

  it('survives two PRs targeting each other', () => {
    const prs = [pr('1', 'a', 'b'), pr('2', 'b', 'a')]
    const entries = groupPrsByStack(prs)
    expect(
      entries.flatMap((entry) =>
        entry.kind === 'single' ? [entry.pr] : entry.prs,
      ),
    ).toHaveLength(2)
  })
})

describe('prCards', () => {
  it('gives each stack its own card and runs of single PRs a shared one', () => {
    const a = pr('1', 'a', 'main')
    const b = pr('2', 'b', 'main')
    const bottom = pr('3', 'feat/a', 'main')
    const top = pr('4', 'feat/b', 'feat/a')
    const c = pr('5', 'c', 'main')
    const cards = prCards(groupPrsByStack([a, b, top, bottom, c]))
    expect(cards).toEqual([
      { kind: 'list', id: '1', prs: [a, b] },
      { kind: 'stack', id: '3', trunk: 'main', prs: [top, bottom] },
      { kind: 'list', id: '5', prs: [c] },
    ])
  })
})
