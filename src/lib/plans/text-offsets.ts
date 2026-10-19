// Comments locate their highlight as character offsets into the rendered plan's text, so they
// survive the markdown being re-rendered (a DOM Range would point at the discarded nodes).

/** The plan's text nodes in order, leaving out controls such as code copy buttons. */
function textNodes(root: HTMLElement): Text[] {
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) =>
      node.parentElement?.closest('button')
        ? NodeFilter.FILTER_REJECT
        : NodeFilter.FILTER_ACCEPT,
  })
  const nodes: Text[] = []
  while (walker.nextNode()) nodes.push(walker.currentNode as Text)
  return nodes
}

function pointOffset(nodes: Text[], container: Node, offset: number): number {
  const point = document.createRange()
  point.setStart(container, offset)
  let position = 0
  for (const node of nodes) {
    if (node === container) return position + offset
    if (point.comparePoint(node, 0) >= 0) return position
    position += node.length
  }
  return position
}

/** Where a selection inside `root` starts and ends, or null when it's empty or outside. */
export function rangeToOffsets(
  root: HTMLElement,
  range: Range,
): { start: number; end: number } | null {
  if (range.collapsed) return null
  if (!root.contains(range.commonAncestorContainer)) return null
  const nodes = textNodes(root)
  const start = pointOffset(nodes, range.startContainer, range.startOffset)
  const end = pointOffset(nodes, range.endContainer, range.endOffset)
  return end > start ? { start, end } : null
}

/** A DOM range over the given text offsets of `root`, or null if the text is shorter. */
export function offsetsToRange(
  root: HTMLElement,
  start: number,
  end: number,
): Range | null {
  const range = document.createRange()
  let position = 0
  let started = false
  for (const node of textNodes(root)) {
    const next = position + node.length
    if (!started && start < next) {
      range.setStart(node, start - position)
      started = true
    }
    if (started && end <= next) {
      range.setEnd(node, end - position)
      return range
    }
    position = next
  }
  return null
}
