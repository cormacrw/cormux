import { describe, expect, it } from 'vitest'
import {
  clampSidebarWidth,
  SIDEBAR_MAX_WIDTH,
  SIDEBAR_MIN_WIDTH,
} from './width'

describe('clampSidebarWidth', () => {
  it('keeps the sidebar between its minimum and maximum width', () => {
    expect(clampSidebarWidth(10)).toBe(SIDEBAR_MIN_WIDTH)
    expect(clampSidebarWidth(9000)).toBe(SIDEBAR_MAX_WIDTH)
    expect(clampSidebarWidth(260.4)).toBe(260)
  })
})
