import { describe, expect, it } from 'vitest'
import type { EngineStatus } from '$lib/ipc/bindings'
import { soleInstalledEngine } from './engines'

function status(kind: EngineStatus['kind'], installed: boolean): EngineStatus {
  return { kind, installed, binary: null, version: null, signedIn: null }
}

describe('soleInstalledEngine', () => {
  it('returns the only installed agent in the picker', () => {
    expect(
      soleInstalledEngine([status('claude', true), status('cursor', false)]),
    ).toBe('claude')
    expect(
      soleInstalledEngine([status('claude', true), status('cursor', true)]),
    ).toBe(null)
    expect(
      soleInstalledEngine([status('claude', false), status('codex', true)]),
    ).toBe(null)
  })
})
