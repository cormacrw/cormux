import { describe, expect, it } from 'vitest'
import { ansiLineToHtml } from './ansi-html'

describe('ansiLineToHtml', () => {
  it('escapes markup in app output', () => {
    const html = ansiLineToHtml('GET /<img src=x onerror=alert(1)>')
    expect(html).not.toContain('<img')
    expect(html).toContain('&lt;img')
  })

  it('still colors ANSI sequences', () => {
    expect(ansiLineToHtml('\u001b[31mfail\u001b[0m')).toContain('<span')
  })
})
