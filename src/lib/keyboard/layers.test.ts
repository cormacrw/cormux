import { describe, expect, it } from 'vitest'
import { keyboardLayers } from './layers.svelte'

describe('keyboardLayers', () => {
  it('closes a slide-out before stopping the agent, whatever registered first', () => {
    const calls: string[] = []
    const offAgent = keyboardLayers.onEscape('agent', () => {
      calls.push('agent')
      return true
    })
    const offSlideOut = keyboardLayers.onEscape('slideOut', () => {
      calls.push('slideOut')
      return true
    })
    expect(keyboardLayers.runEscape()).toBe(true)
    expect(calls).toEqual(['slideOut'])

    offSlideOut()
    expect(keyboardLayers.runEscape()).toBe(true)
    expect(calls).toEqual(['slideOut', 'agent'])
    offAgent()
  })

  it('falls through an Esc step with nothing to do', () => {
    const off = keyboardLayers.onEscape('slideOut', () => false)
    expect(keyboardLayers.runEscape()).toBe(false)
    off()
  })

  it('holds the keyboard only while a layer is open, and only a modal makes shortcuts wait', () => {
    let menuOpen = true
    const releaseMenu = keyboardLayers.hold('overlay', () => menuOpen)
    expect(keyboardLayers.held).toBe(true)
    expect(keyboardLayers.modal).toBe(false)

    let dialogOpen = true
    const releaseDialog = keyboardLayers.hold('modal', () => dialogOpen)
    expect(keyboardLayers.modal).toBe(true)

    // Read live: a layer that has just closed no longer counts, before it's released.
    menuOpen = false
    dialogOpen = false
    expect(keyboardLayers.held).toBe(false)
    releaseMenu()
    releaseDialog()
  })

  it('keeps one popover open at a time, and Esc closes it with focus handed back', () => {
    const calls: string[] = []
    const popover = (name: string) => ({
      close: () => calls.push(`close ${name}`),
      restoreFocus: () => calls.push(`focus ${name}`),
    })
    const first = popover('first')
    const second = popover('second')
    keyboardLayers.claimPopover(first)
    keyboardLayers.claimPopover(second)
    expect(calls).toEqual(['close first'])

    expect(keyboardLayers.closePopover()).toBe(true)
    expect(calls).toEqual(['close first', 'close second', 'focus second'])
    keyboardLayers.releasePopover(second)
    expect(keyboardLayers.closePopover()).toBe(false)
  })

  it('closes the popover and what gives way to a dialog as one opens', () => {
    const calls: string[] = []
    const popover = {
      close: () => calls.push('close popover'),
      restoreFocus: () => calls.push('focus'),
    }
    keyboardLayers.claimPopover(popover)
    const release = keyboardLayers.closeUnderModal(() =>
      calls.push('close palette'),
    )
    keyboardLayers.modalOpening()
    expect(calls).toEqual(['close popover', 'focus', 'close palette'])
    keyboardLayers.releasePopover(popover)
    release()
  })
})
