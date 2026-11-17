import { type ReactNode, type SyntheticEvent, useEffect, useId, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import {
  BasketIcon,
  Button,
  CopyIcon,
  IconButton,
  LeaderRow,
  OpenIcon,
  RevealIcon,
  TerminalIcon,
  Tooltip,
  TrashIcon,
} from '../components/ui'
import { formatBytes, formatCount } from '../lib/format'
import { type ExportFormat, type NodeEntry, type ScanId, getAncestors } from '../lib/ipc'
import type { BasketItem } from '../store'
import type { Os } from '../useOs'
import { useT } from '../useT'
import type { ActionTarget, Actions, MenuAt } from './useBrowseActions'

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
      <div role="group" aria-label={t('actions.label')} className="flex flex-wrap items-center gap-2">
        <IconButton
          label={t(REVEAL_TEXT[os ?? 'other'])}
          keys="R"
          icon={<RevealIcon />}
          side="top"
          align="start"
          onClick={actions.reveal}
          aria-describedby="browse-action-target"
          aria-keyshortcuts="R"
        />
        <IconButton
          label={t('actions.open')}
          keys="O"
          icon={<OpenIcon />}
          side="top"
          onClick={actions.open}
          aria-describedby="browse-action-target"
          aria-keyshortcuts="O"
        />
        <IconButton
          label={t('actions.copy')}
          keys={mac ? '⌘C' : 'Ctrl+C'}
          icon={<CopyIcon />}
          side="top"
          onClick={actions.copy}
          aria-describedby="browse-action-target"
          aria-keyshortcuts={mac ? 'Meta+C' : 'Control+C'}
        />
        <IconButton
          label={t('actions.terminal')}
          keys="T"
          icon={<TerminalIcon />}
          side="top"
          onClick={actions.terminal}
          aria-describedby="browse-action-target"
          aria-keyshortcuts="T"
        />
        <span aria-hidden className="mx-1 h-5 w-px bg-hairline" />
        <Tooltip side="top" text={t('actions.key')} keys={t('basket.key')}>
          <Button
            onClick={onBasket}
            disabled={!canTrash}
            title={canTrash ? undefined : t('basket.root')}
            aria-describedby="browse-action-target"
            aria-keyshortcuts="Space"
          >
            <BasketIcon />
            {t(inBasket ? 'basket.remove' : 'basket.add')}
          </Button>
        </Tooltip>
        <Tooltip side="top" text={t('actions.key')} keys={mac ? '⌘⌫' : 'Delete'}>
          <Button
            onClick={onTrash}
            disabled={!canTrash}
            title={canTrash ? undefined : t('actions.trashRoot')}
            aria-describedby="browse-action-target"
            aria-keyshortcuts={mac ? 'Meta+Backspace' : 'Delete'}
            aria-haspopup="dialog"
          >
            <TrashIcon />
            {t('actions.trash')}
          </Button>
        </Tooltip>
        {permanent && (
          <Tooltip side="top" text={t('actions.key')} keys={mac ? '⌥⌘⌫' : 'Shift+Delete'}>
            <Button
              onClick={onDelete}
              disabled={!canTrash}
              title={canTrash ? undefined : t('actions.deleteRoot')}
              aria-describedby="browse-action-target"
              aria-keyshortcuts={mac ? 'Alt+Meta+Backspace' : 'Shift+Delete'}
              aria-haspopup="dialog"
            >
              {t('actions.delete')}
            </Button>
          </Tooltip>
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


/**
 * The item menu: the action row's actions at the pointer, for the item just
 * right-clicked (or the selected one, from the keyboard). ↑ ↓ move, Enter
 * chooses, Esc or a click elsewhere closes and gives focus back.
 */
export function ItemMenu({
  at,
  onClose,
  actions,
  os,
  canTrash,
  onTrash,
  inBasket,
  onBasket,
  permanent,
  onDelete,
  onExport,
}: {
  at: MenuAt
  onClose: () => void
  actions: Actions
  os: Os | null
  canTrash: boolean
  onTrash: () => void
  inBasket: boolean
  onBasket: () => void
  permanent: boolean
  onDelete: () => void
  /** Writes the item and everything below it; the save dialog asks where. */
  onExport: (format: ExportFormat) => void
}) {
  const { t } = useT()
  const mac = os === 'macos'
  const menu = useRef<HTMLDivElement>(null)
  const [pos, setPos] = useState(at)

  const items: { key: string; text: string; kbd: string; run: () => void; off?: boolean }[] = [
    { key: 'reveal', text: t(REVEAL_TEXT[os ?? 'other']), kbd: 'R', run: actions.reveal },
    { key: 'open', text: t('actions.open'), kbd: 'O', run: actions.open },
    { key: 'copy', text: t('actions.copy'), kbd: mac ? '⌘C' : 'Ctrl+C', run: actions.copy },
    { key: 'terminal', text: t('actions.terminal'), kbd: 'T', run: actions.terminal },
    { key: 'csv', text: t('export.itemCsv'), kbd: '', run: () => onExport('csv') },
    { key: 'json', text: t('export.itemJson'), kbd: '', run: () => onExport('json') },
    { key: 'basket', text: t(inBasket ? 'basket.remove' : 'basket.add'), kbd: t('basket.key'), run: onBasket, off: !canTrash },
    { key: 'trash', text: t('actions.trash'), kbd: mac ? '⌘⌫' : 'Delete', run: onTrash, off: !canTrash },
    ...(permanent
      ? [{ key: 'delete', text: t('actions.delete'), kbd: mac ? '⌥⌘⌫' : 'Shift+Delete', run: onDelete, off: !canTrash }]
      : []),
  ]

  const buttons = () => Array.from(menu.current?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)') ?? [])

  // Keep it inside the window, then focus the first item.
  useEffect(() => {
    const el = menu.current
    if (el) {
      const r = el.getBoundingClientRect()
      const x = Math.max(8, Math.min(at.x, window.innerWidth - r.width - 8))
      const y = Math.max(8, Math.min(at.y, window.innerHeight - r.height - 8))
      setPos({ ...at, x, y })
    }
    buttons()[0]?.focus()
  }, [at])

  useEffect(() => {
    const away = (e: PointerEvent) => {
      if (!menu.current?.contains(e.target as Node)) onClose()
    }
    const blur = () => onClose()
    document.addEventListener('pointerdown', away, true)
    window.addEventListener('blur', blur)
    window.addEventListener('resize', blur)
    return () => {
      document.removeEventListener('pointerdown', away, true)
      window.removeEventListener('blur', blur)
      window.removeEventListener('resize', blur)
    }
  }, [onClose])

  const close = () => {
    onClose()
    at.from?.focus()
  }

  const onKeyDown = (e: SyntheticEvent<HTMLDivElement> & { key: string }) => {
    const list = buttons()
    const i = list.indexOf(document.activeElement as HTMLButtonElement)
    const next = { ArrowDown: i + 1, ArrowUp: i - 1, Home: 0, End: list.length - 1 }[e.key]
    if (next !== undefined) list[(next + list.length) % list.length]?.focus()
    else if (e.key === 'Escape' || e.key === 'Tab') close()
    else return
    e.preventDefault()
    e.stopPropagation()
  }

  // Portalled to <body>: a transformed ancestor (screen enter motion) would
  // otherwise become the containing block of `fixed` and shift the menu.
  return createPortal(
    <div
      ref={menu}
      role="menu"
      aria-label={t('actions.menu')}
      onKeyDown={onKeyDown}
      onContextMenu={(e) => e.preventDefault()}
      style={{ left: pos.x, top: pos.y }}
      className="fixed z-50 flex min-w-56 origin-top-left flex-col rounded-card border border-card-edge bg-card p-1 text-body shadow-float enter-pop"
    >
      {items.map((item) => (
        <button
          key={item.key}
          type="button"
          role="menuitem"
          disabled={item.off}
          onClick={() => {
            close()
            item.run()
          }}
          className="flex h-8 cursor-pointer items-center justify-between gap-6 rounded-control px-3 text-left text-ink outline-none hover:bg-raised focus-visible:bg-raised disabled:cursor-default disabled:opacity-50 disabled:hover:bg-transparent"
        >
          {item.text}
          <Kbd>{item.kbd}</Kbd>
        </button>
      ))}
    </div>,
    document.body,
  )
}
