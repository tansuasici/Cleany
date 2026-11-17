import { type ReactNode, type SyntheticEvent, useEffect, useId, useRef, useState } from 'react'
import { Button, LeaderRow } from '../components/ui'
import { formatBytes, formatCount } from '../lib/format'
import { type NodeEntry, type ScanId, getAncestors } from '../lib/ipc'
import type { BasketItem } from '../store'
import type { Os } from '../useOs'
import { useT } from '../useT'
import type { ActionTarget, Actions } from './useBrowseActions'

const REVEAL_TEXT = { macos: 'actions.revealMac', windows: 'actions.revealWin', other: 'actions.revealOther' } as const

const Kbd = ({ children }: { children: string }) => (
  <kbd aria-hidden className="font-mono text-meta font-normal text-ink-3">
    {children}
  </kbd>
)

/** How a confirmed removal leaves the disk: through the OS trash, or for good. */
export type Removal = 'trash' | 'delete'

/** What the last move to the trash came to; `alert` lists what did not go. */
export interface TrashNote {
  status: string
  alert: string
}

/** The actions under the view, with their feedback lines. */
export function ActionBar({
  target,
  actions,
  os,
  canTrash,
  onTrash,
  inBasket,
  onBasket,
  permanent,
  onDelete,
  note,
}: {
  target: ActionTarget
  actions: Actions
  os: Os | null
  /** Off for the scan root; it can go neither to the trash nor to the basket. */
  canTrash: boolean
  onTrash: () => void
  inBasket: boolean
  onBasket: () => void
  /** Settings allow permanent delete, so its button is shown. */
  permanent: boolean
  onDelete: () => void
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
        <Button
          onClick={onBasket}
          disabled={!canTrash}
          title={canTrash ? undefined : t('basket.root')}
          aria-describedby="browse-action-target"
          aria-keyshortcuts="Space"
        >
          {t(inBasket ? 'basket.remove' : 'basket.add')}
          <Kbd>{t('basket.key')}</Kbd>
        </Button>
        {permanent && (
          <Button
            onClick={onDelete}
            disabled={!canTrash}
            title={canTrash ? undefined : t('actions.deleteRoot')}
            aria-describedby="browse-action-target"
            aria-keyshortcuts={mac ? 'Alt+Meta+Backspace' : 'Shift+Delete'}
            aria-haspopup="dialog"
          >
            {t('actions.delete')}
            <Kbd>{mac ? '⌥⌘⌫' : 'Shift+Delete'}</Kbd>
          </Button>
        )}
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
 * The frame both trash questions share. A native modal: focus stays inside,
 * Escape cancels (not while moving), and focus goes back to where it was if
 * that is still on the page. Opened by key as often as by mouse, so it
 * appears without animation.
 */
function TrashConfirm({
  removal,
  title,
  children,
  working,
  onCancel,
  onConfirm,
}: {
  removal: Removal
  title: string
  children: ReactNode
  working: boolean
  onCancel: () => void
  onConfirm: () => void
}) {
  const { t, locale } = useT()
  const ref = useRef<HTMLDialogElement>(null)
  const titleId = useId()
  const detailId = useId()
  const wordId = useId()
  const [typed, setTyped] = useState('')
  const word = t('delete.word')
  // A permanent delete waits for the word typed out; a move to the trash does not.
  const ready = removal === 'trash' || typed.trim().toLocaleLowerCase(locale) === word

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

  const cancel = (e: SyntheticEvent) => {
    e.preventDefault()
    if (!working) onCancel()
  }

  return (
    <dialog
      ref={ref}
      aria-labelledby={titleId}
      aria-describedby={detailId}
      onCancel={cancel}
      className="m-auto w-[calc(100%-2rem)] max-w-104 rounded-panel border border-dialog-edge bg-card p-6 text-ink shadow-dialog backdrop:bg-deepest/40"
    >
      <h2 id={titleId} className="text-strong font-medium break-words">
        {title}
      </h2>
      <div id={detailId} className="mt-2 flex flex-col text-body text-ink-2">
        {children}
        <p className="mt-4">{t(removal === 'trash' ? 'trash.undo' : 'delete.warning')}</p>
      </div>
      <form
        onSubmit={(e) => {
          e.preventDefault()
          if (ready && !working) onConfirm()
        }}
      >
        {removal === 'delete' && (
          <div className="mt-4 flex flex-col gap-1">
            <label htmlFor={wordId} className="text-body text-ink">
              {t('delete.prompt', { word })}
            </label>
            {/* Focus lands here: typing is the way forward, Escape the way out. */}
            <input
              id={wordId}
              value={typed}
              onChange={(e) => setTyped(e.target.value)}
              disabled={working}
              autoFocus
              autoComplete="off"
              autoCapitalize="off"
              autoCorrect="off"
              spellCheck={false}
              className="h-8 rounded-control border border-edge bg-card px-2 text-body text-ink"
            />
          </div>
        )}
        <div className="mt-6 flex flex-wrap justify-end gap-2">
          {/* First in the dialog, so focus lands on the safe choice. */}
          <Button type="button" onClick={onCancel} disabled={working} autoFocus={removal === 'trash'}>
            {t('trash.cancel')}
            <Kbd>Esc</Kbd>
          </Button>
          <Button type="submit" variant="primary" disabled={working || !ready} aria-busy={working}>
            {removal === 'trash'
              ? working
                ? t('trash.working')
                : t('trash.confirm')
              : working
                ? t('delete.working')
                : t('delete.confirm')}
          </Button>
        </div>
      </form>
    </dialog>
  )
}

/** Asks before moving `target` to the trash. */
export function TrashDialog({
  removal,
  scan,
  target,
  working,
  onCancel,
  onConfirm,
}: {
  removal: Removal
  scan: ScanId
  target: ActionTarget
  working: boolean
  onCancel: () => void
  onConfirm: () => void
}) {
  const { t, locale } = useT()
  const [entry, setEntry] = useState<NodeEntry | null>(null)

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

  const detail =
    entry === null
      ? ''
      : entry.kind === 'dir'
        ? t('trash.folder', {
            files: t('browse.fileCount', { n: formatCount(entry.files, locale), count: entry.files }),
          })
        : t('trash.file')

  return (
    <TrashConfirm
      removal={removal}
      title={t(removal === 'trash' ? 'trash.title' : 'delete.title', { name: target.name })}
      working={working}
      onCancel={onCancel}
      onConfirm={onConfirm}
    >
      <p className="min-h-5 tabular-nums">{detail}</p>
      {/* The döküm's line, once: what this move gives back. */}
      <p className="mt-4 min-h-5 text-ink">
        {entry && <LeaderRow name={t('trash.space')} value={formatBytes(entry.allocated, locale)} strong />}
      </p>
    </TrashConfirm>
  )
}

/** How many basket items the question names before "+k more". */
const LISTED = 5

/** Asks before moving every basket item to the trash. */
export function BasketTrashDialog({
  removal,
  items,
  working,
  onCancel,
  onConfirm,
}: {
  removal: Removal
  items: BasketItem[]
  working: boolean
  onCancel: () => void
  onConfirm: () => void
}) {
  const { t, locale } = useT()
  const count = items.length
  const n = formatCount(count, locale)
  const total = items.reduce((sum, item) => sum + item.allocated, 0)
  const rest = count - LISTED
  return (
    <TrashConfirm
      removal={removal}
      title={t(removal === 'trash' ? 'basket.trashTitle' : 'delete.basketTitle', { n, count })}
      working={working}
      onCancel={onCancel}
      onConfirm={onConfirm}
    >
      <ul className="flex flex-col">
        {items.slice(0, LISTED).map((item) => (
          <li key={item.id} className="h-6 leading-6">
            <LeaderRow name={item.name} title={item.name} value={formatBytes(item.allocated, locale)} />
          </li>
        ))}
        {rest > 0 && (
          <li className="h-6 leading-6">{t('basket.more', { n: formatCount(rest, locale), count: rest })}</li>
        )}
      </ul>
      <p className="mt-4 text-ink">
        <LeaderRow name={t('trash.space')} value={formatBytes(total, locale)} strong />
      </p>
    </TrashConfirm>
  )
}
