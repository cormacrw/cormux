/** The row showing `line` of the new file in a rendered diff, in unified or split mode. */
function findNewLineRow(
  fileEl: Element,
  line: number,
): HTMLTableRowElement | null {
  const unified = fileEl.querySelector(`[data-line-new-num="${line}"]`)
  if (unified) return unified.closest('tr')
  for (const cell of fileEl.querySelectorAll(`[data-line-num="${line}"]`)) {
    if (cell.closest('[data-side]')?.getAttribute('data-side') === 'new') {
      return cell.closest('tr')
    }
  }
  return null
}

/**
 * Scrolls `scroller` alone so `el` sits at its top or centre. `scrollIntoView` would also scroll
 * any clipped ancestor, such as the workspace's row while the tray is still sliding in.
 */
export function scrollWithin(
  scroller: HTMLElement,
  el: Element,
  block: 'start' | 'center',
) {
  const box = scroller.getBoundingClientRect()
  const target = el.getBoundingClientRect()
  const offset =
    block === 'start'
      ? target.top - box.top
      : target.top + target.height / 2 - (box.top + box.height / 2)
  scroller.scrollTop += offset
}

/**
 * Scrolls the diff to a line of the new file and flashes it. The diff renders after the
 * file expands, so this retries for a moment. Lines outside the diff's hunks aren't
 * shown at all; then the file header stays in view instead.
 */
export function revealDiffLine(
  scroller: HTMLElement,
  fileEl: Element,
  line: number,
  timeoutMs = 1500,
) {
  const startedAt = performance.now()
  const attempt = () => {
    const row = findNewLineRow(fileEl, line)
    if (!row) {
      if (performance.now() - startedAt < timeoutMs)
        requestAnimationFrame(attempt)
      return
    }
    scrollWithin(scroller, row, 'center')
    // Cells paint their own backgrounds, so tint each one rather than the row.
    for (const cell of row.cells) {
      cell.animate(
        [
          { boxShadow: 'inset 0 0 0 100vmax rgb(250 204 21 / 0.35)' },
          { boxShadow: 'inset 0 0 0 100vmax rgb(250 204 21 / 0)' },
        ],
        { duration: 1800, easing: 'ease-out' },
      )
    }
  }
  attempt()
}
