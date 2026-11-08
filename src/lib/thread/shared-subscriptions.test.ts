import { describe, expect, it } from 'vitest'
import { SharedSubscriptions } from './shared-subscriptions'

/** A stream per key that records what it delivers, and whether it's still open. */
function streams() {
  const open = new Map<string, (event: string) => void>()
  const received: string[] = []
  const subscriptions = new SharedSubscriptions((key) => {
    const listener = (event: string) => received.push(`${key}:${event}`)
    open.set(key, listener)
    return () => {
      if (open.get(key) === listener) open.delete(key)
    }
  })
  const send = (key: string, event: string) => open.get(key)?.(event)
  return { subscriptions, send, received, open }
}

describe('shared subscriptions', () => {
  it('keeps every watched thread streaming, not just the newest', () => {
    const { subscriptions, send, received } = streams()
    subscriptions.watch('a')
    subscriptions.watch('b')
    send('a', 'one')
    send('b', 'two')
    expect(received).toEqual(['a:one', 'b:two'])
  })

  it('leaves the others streaming when one stops', () => {
    const { subscriptions, send, received } = streams()
    const stopA = subscriptions.watch('a')
    subscriptions.watch('b')
    stopA()
    send('a', 'gone')
    send('b', 'still here')
    expect(received).toEqual(['b:still here'])
    expect(subscriptions.watched()).toEqual(new Set(['b']))
  })

  it('shares one stream between two watchers of a thread until both stop', () => {
    const { subscriptions, open } = streams()
    const first = subscriptions.watch('a')
    const second = subscriptions.watch('a')
    expect(open.size).toBe(1)
    first()
    first()
    expect(open.has('a')).toBe(true)
    second()
    expect(open.has('a')).toBe(false)
  })
})
