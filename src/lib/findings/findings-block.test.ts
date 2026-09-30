import { describe, expect, it } from 'vitest'
import { hideFindingsBlock } from './findings-block'

describe('hideFindingsBlock', () => {
  it('leaves ordinary replies alone', () => {
    expect(hideFindingsBlock('Looks good to me.')).toBe('Looks good to me.')
  })

  it('removes a complete block and the whitespace before it', () => {
    expect(
      hideFindingsBlock(
        'Two issues.\n\n<cormux-findings>\n[{"title":"x"}]\n</cormux-findings>',
      ),
    ).toBe('Two issues.')
  })

  it('hides a block that is still streaming', () => {
    expect(hideFindingsBlock('Two issues.\n<cormux-findings>\n[{"sev')).toBe(
      'Two issues.',
    )
  })

  it('hides a partial opening tag at the end', () => {
    expect(hideFindingsBlock('Two issues.\n<cormux-fi')).toBe('Two issues.')
  })

  it('keeps text after the closing tag', () => {
    expect(
      hideFindingsBlock('Done.<cormux-findings>[]</cormux-findings>\nThanks!'),
    ).toBe('Done.\nThanks!')
  })
})
