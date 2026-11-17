import { type KeyboardEvent, useCallback, useEffect, useState } from 'react'
import { type ActionError, type ScanId, getPath, openNode, revealNode } from '../lib/ipc'
import { useT } from '../useT'

/** What the action row acts on: the selected item, or the folder on screen. */
export interface ActionTarget {
  id: number
  name: string
}

/** "Kopyalandı" stays up this long. */
const COPIED_MS = 2000

type Feedback = { target: number; text: string; kind: 'copied' | 'error' } | null

/**
 * Show in the file manager, open, and copy the path, for `target`. The path is
 * fetched ahead on every target change so the clipboard write happens inside
 * the click or keypress (WebKit refuses it after an await).
 */
export function useBrowseActions(scan: ScanId, target: ActionTarget) {
  const { t } = useT()
  const [path, setPath] = useState<{ id: number; path: string } | null>(null)
  const [feedback, setFeedback] = useState<Feedback>(null)

  useEffect(() => {
    let live = true
    getPath(scan, target.id).then(
      (p) => live && setPath({ id: target.id, path: p }),
      () => live && setPath(null),
    )
    return () => {
      live = false
    }
  }, [scan, target.id])

  useEffect(() => {
    if (feedback?.kind !== 'copied') return
    const timer = setTimeout(() => setFeedback(null), COPIED_MS)
    return () => clearTimeout(timer)
  }, [feedback])

  const id = target.id
  const fail = useCallback(
    (fallback: string) => (e: unknown) => {
      const gone = (e as ActionError | undefined)?.kind === 'gone'
      setFeedback({ target: id, kind: 'error', text: gone ? t('actions.gone') : fallback })
    },
    [id, t],
  )

  const reveal = useCallback(() => {
    setFeedback(null)
    revealNode(scan, id).catch(fail(t('actions.revealError')))
  }, [scan, id, fail, t])

  const open = useCallback(() => {
    setFeedback(null)
    openNode(scan, id).catch(fail(t('actions.openError')))
  }, [scan, id, fail, t])

  const copy = useCallback(() => {
    const write = (p: string) =>
      navigator.clipboard.writeText(p).then(
        () => setFeedback({ target: id, kind: 'copied', text: t('actions.copied') }),
        fail(t('actions.copyError')),
      )
    if (path?.id === id) void write(path.path)
    else getPath(scan, id).then(write, fail(t('actions.copyError')))
  }, [scan, id, path, fail, t])

  // Feedback belongs to the item it was about.
  const shown = feedback?.target === id ? feedback : null
  return { reveal, open, copy, feedback: shown }
}

export type Actions = ReturnType<typeof useBrowseActions>

/** Controls that Space already presses; the basket key leaves them alone. */
const PRESSABLE = 'button, a[href], select, summary, [role="button"], [role="radio"], [role="checkbox"], [role="switch"], [role="tab"], [role="menuitem"]'

/**
 * R shows, O opens, ⌘C / Ctrl+C copies the path, Space puts the target in the
 * basket or takes it out. Bound on the Browse section so every view gets
 * them; the views leave these keys alone.
 */
export function actionKeys(actions: Actions, toggleBasket: () => void) {
  return (e: KeyboardEvent<HTMLElement>) => {
    if (e.defaultPrevented || e.altKey || e.shiftKey) return
    const el = e.target as HTMLElement
    if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable) return
    // Keys in a dialog belong to it.
    if (el.closest('dialog')) return
    const mod = e.metaKey || e.ctrlKey
    const key = e.key.toLowerCase()
    if (!mod && (key === 'r' || key === 'o')) {
      // Holding the key must not open a window per repeat.
      if (!e.repeat) (key === 'r' ? actions.reveal : actions.open)()
    } else if (!mod && e.key === ' ') {
      if (el.closest(PRESSABLE)) return
      if (!e.repeat) toggleBasket()
    } else if (mod && key === 'c') {
      // Selected text copies as usual.
      if (window.getSelection()?.toString()) return
      actions.copy()
    } else {
      return
    }
    e.preventDefault()
  }
}

/**
 * ⌘⌫ on macOS, Delete elsewhere: asks to move the target to the trash. Bound
 * in the capture phase so the views never read ⌘⌫ as Backspace (up a folder).
 */
export function trashKey(mac: boolean, ask: () => void) {
  return (e: KeyboardEvent<HTMLElement>) => {
    const el = e.target as HTMLElement
    if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable) return
    if (el.closest('dialog')) return
    if (e.altKey || e.shiftKey) return
    const hit = mac
      ? e.key === 'Backspace' && e.metaKey && !e.ctrlKey
      : e.key === 'Delete' && !e.metaKey && !e.ctrlKey
    if (!hit) return
    e.preventDefault()
    e.stopPropagation()
    if (!e.repeat) ask()
  }
}

/**
 * ⌥⌘⌫ on macOS, Shift+Delete elsewhere: asks to delete the target for good.
 * Bound beside `trashKey`; the caller binds it only while Settings allow it.
 */
export function deleteKey(mac: boolean, ask: () => void) {
  return (e: KeyboardEvent<HTMLElement>) => {
    const el = e.target as HTMLElement
    if (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable) return
    if (el.closest('dialog')) return
    const hit = mac
      ? e.key === 'Backspace' && e.metaKey && e.altKey && !e.ctrlKey && !e.shiftKey
      : e.key === 'Delete' && e.shiftKey && !e.metaKey && !e.ctrlKey && !e.altKey
    if (!hit) return
    e.preventDefault()
    e.stopPropagation()
    if (!e.repeat) ask()
  }
}
