import { type SyntheticEvent, useEffect, useId, useRef, useState } from 'react'
import { Button } from '../components/ui'
import { formatBytes, formatCount } from '../lib/format'
import { type NodeEntry, type ScanId, getAncestors } from '../lib/ipc'
import type { Os } from '../useOs'
import { useT } from '../useT'
import type { ActionTarget, Actions } from './useBrowseActions'

const REVEAL_TEXT = { macos: 'actions.revealMac', windows: 'actions.revealWin', other: 'actions.revealOther' } as const

const Kbd = ({ children }: { children: string }) => (
  <kbd aria-hidden className="font-mono text-meta font-normal text-ink-3">
    {children}
  </kbd>
)

/** What the last move to the trash came to; `alert` lists what did not go. */
export interface TrashNote {
  status: string
  alert: string
}

/** The four actions under the view, with their feedback lines. */
export function ActionBar({
  target,
  actions,
  os,
  canTrash,
  onTrash,
  note,
}: {
  target: ActionTarget
  actions: Actions
  os: Os | null
  canTrash: boolean
  onTrash: () => void
  note: TrashNote | null
}) {
  const { t } = useT()
  const mac = os === 'macos'
  const { feedback } = actions
  const status = feedback?.kind === 'copied' ? feedback.text : (note?.status ?? '')
  const alert = feedback?.kind === 'error' ? feedback.text : (note?.alert ?? '')
  return (
    <div className="flex flex-wrap items-center gap-2">
      <span id="browse-action-target" className="sr-only">
        {t('actions.target', { name: target.name })}
      </span>
      <div role="group" aria-label={t('actions.label')} className="flex flex-wrap gap-2">
        <Button onClick={actions.reveal} aria-describedby="browse-action-target" aria-keyshortcuts="R">
          {t(REVEAL_TEXT[os ?? 'other'])}
          <Kbd>R</Kbd>
        </Button>
        <Button onClick={actions.open} aria-describedby="browse-action-target" aria-keyshortcuts="O">
          {t('actions.open')}
          <Kbd>O</Kbd>
        </Button>
        <Button
          onClick={actions.copy}
          aria-describedby="browse-action-target"
          aria-keyshortcuts={mac ? 'Meta+C' : 'Control+C'}
        >
          {t('actions.copy')}
          <Kbd>{mac ? '⌘C' : 'Ctrl+C'}</Kbd>
        </Button>
        <Button
          onClick={onTrash}
          disabled={!canTrash}
          title={canTrash ? undefined : t('actions.trashRoot')}
          aria-describedby="browse-action-target"
          aria-keyshortcuts={mac ? 'Meta+Backspace' : 'Delete'}
          aria-haspopup="dialog"
        >
          {t('actions.trash')}
          <Kbd>{mac ? '⌘⌫' : 'Delete'}</Kbd>
        </Button>
      </div>
      {/* Both regions stay mounted so a change is announced. Shown at once:
          the feedback answers a keypress, and keys don't animate. */}
      <p role="status" className="text-secondary text-ink-2">
        {status}
      </p>
      <p role="alert" className="text-secondary text-ink">
        {alert}
      </p>
    </div>
  )
}

/**
 * Asks before moving `target` to the trash. A native modal: focus stays
 * inside, Escape cancels (not while moving), and focus goes back to where it
 * was if that is still on the page. Opened by key as often as by mouse, so it
 * appears without animation.
 */
export function TrashDialog({
  scan,
  target,
  working,
  onCancel,
  onConfirm,
}: {
  scan: ScanId
  target: ActionTarget
  working: boolean
  onCancel: () => void
  onConfirm: () => void
}) {
  const { t, locale } = useT()
  const ref = useRef<HTMLDialogElement>(null)
  const titleId = useId()
  const detailId = useId()
  const [entry, setEntry] = useState<NodeEntry | null>(null)

  useEffect(() => {
    const dialog = ref.current
    const before = document.activeElement as HTMLElement | null
    dialog?.showModal()
    return () => {
      dialog?.close()
      // A view that was rebuilt focuses itself; otherwise go back.
      if (before?.isConnected) before.focus()
    }
  }, [])

  // Size and file count, from the tree as it is now.
  useEffect(() => {
    let live = true
    getAncestors(scan, target.id).then(
      (list) => live && setEntry(list[list.length - 1] ?? null),
      () => undefined,
    )
    return () => {
      live = false
    }
  }, [scan, target.id])

  const cancel = (e: SyntheticEvent) => {
    e.preventDefault()
    if (!working) onCancel()
  }

  const detail =
    entry === null
      ? ''
      : entry.kind === 'dir'
        ? t('trash.folder', {
            size: formatBytes(entry.allocated, locale),
            files: t('browse.fileCount', { n: formatCount(entry.files, locale), count: entry.files }),
          })
        : t('trash.file', { size: formatBytes(entry.allocated, locale) })

  return (
    <dialog
      ref={ref}
      aria-labelledby={titleId}
      aria-describedby={detailId}
      onCancel={cancel}
      className="m-auto w-[calc(100%-2rem)] max-w-104 rounded-panel border border-hairline bg-card p-6 text-ink backdrop:bg-deepest/40"
    >
      <h2 id={titleId} className="text-strong font-medium break-words">
        {t('trash.title', { name: target.name })}
      </h2>
      <div id={detailId} className="mt-2 flex flex-col gap-1 text-body text-ink-2">
        <p className="min-h-5 font-mono tabular-nums">{detail}</p>
        <p>{t('trash.undo')}</p>
      </div>
      <div className="mt-6 flex flex-wrap justify-end gap-2">
        {/* First in the dialog, so focus lands on the safe choice. */}
        <Button onClick={onCancel} disabled={working} autoFocus>
          {t('trash.cancel')}
        </Button>
        <Button variant="primary" onClick={onConfirm} disabled={working} aria-busy={working}>
          {working ? t('trash.working') : t('trash.confirm')}
        </Button>
      </div>
    </dialog>
  )
}
