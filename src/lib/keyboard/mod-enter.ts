/** ⌘↵ (Ctrl+↵ elsewhere): submits a dialog or comment box, as its `DialogShortcut` hint says. */
export function isModEnter(event: KeyboardEvent) {
  return event.key === 'Enter' && (event.metaKey || event.ctrlKey)
}

export function onModEnter(submit: () => void) {
  return (event: KeyboardEvent) => {
    if (!isModEnter(event)) return
    event.preventDefault()
    submit()
  }
}
