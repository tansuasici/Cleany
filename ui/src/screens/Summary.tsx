import { useEffect, useState } from 'react'
import { Button, ChevronIcon, LeaderRow, NAME, Named, ShareBar } from '../components/ui'
import { formatBytes, formatCount, formatDuration, formatShare, truncateMiddle } from '../lib/format'
import { type NodeEntry, type ScanId, getChildren, openDiskAccessSettings } from '../lib/ipc'
import { type DoneScreen, useStore } from '../store'
import { useT } from '../useT'

type Props = { screen: DoneScreen }

/** How many children an opened folder lists in place. */
const CHILD_ITEMS = 8
/** Names longer than this are cut in the middle; the full name is the tooltip. */
const NAME_CHARS = 44
/** Below this share of the total a bar would be a speck, so none is drawn. */
const MIN_BAR_SHARE = 0.005

/** One grid for every döküm line: name ··· size | bar | share | open (narrow: name | share | open, bar below). */
const LINE =
  'grid grid-cols-[minmax(0,1fr)_3rem_2.5rem] items-baseline gap-x-2 sm:grid-cols-[minmax(0,1fr)_7.5rem_3.5rem_3rem] sm:gap-x-4'
/** Share sits after the name on narrow screens, after the bar from `sm` up; the bar drops under the name. */
const SHARE_CELL = 'col-start-2 row-start-1 text-right sm:col-start-3'

/** Moment 2: the end-of-scan döküm — items with their share, one subtotal, footnotes. */
export function SummaryScreen({ screen }: Props) {
  const { t, locale } = useT()
  const scan = useStore((s) => s.scan)
  const browse = useStore((s) => s.browse)
  const noAccess = useStore((s) => s.access === 'denied')
  const { target, summary, top } = screen
  const { root, stats, issues, issueCount } = summary
  const bytes = (n: number) => formatBytes(n, locale)
  const count = (n: number) => ({ n: formatCount(n, locale), count: n })

  const notes = [
    stats.hardlinks > 0 && t('summary.hardlinks', count(stats.hardlinks)),
    stats.placeholders > 0 && t('summary.placeholders', count(stats.placeholders)),
    stats.skippedMounts > 0 && t('summary.skippedMounts', count(stats.skippedMounts)),
  ].filter((note): note is string => typeof note === 'string')

  const max = top.status === 'ready' ? (top.items[0]?.allocated ?? 0) : 0

  return (
    <section
      aria-labelledby="summary-title"
      className="mx-auto w-full max-w-156 px-4 py-10 sm:px-8 min-[1100px]:grid min-[1100px]:max-w-240 min-[1100px]:grid-cols-[minmax(0,35rem)_17.5rem] min-[1100px]:gap-x-8"
    >
      <header className="min-[1100px]:col-span-2">
        <h1 id="summary-title" className="text-title font-medium">
          <Named text={t('summary.title', { name: NAME })} name={target.name} />
        </h1>
        {target.free !== undefined && (
          <p className="mt-1 text-ink-2 tabular-nums">{t('summary.free', { free: bytes(target.free) })}</p>
        )}
      </header>

      <article className="mt-8 rounded-panel border border-hairline bg-card p-4 sm:p-6">
        <div className={`${LINE} text-secondary text-ink-2`}>
          <h2 className="col-start-1 row-start-1 pl-5">{t('summary.items')}</h2>
          <span className={SHARE_CELL}>{t('summary.share')}</span>
        </div>
        <div className="mt-3">
          {top.status === 'loading' && (
            <p role="status" className="text-ink-2">
              {t('summary.loadingItems')}
            </p>
          )}
          {top.status === 'error' && (
            <p role="status" className="text-ink-2">
              {t('summary.itemsError')}
            </p>
          )}
          {top.status === 'ready' && top.items.length === 0 && <p className="text-ink-2">{t('summary.noItems')}</p>}
          {top.status === 'ready' && top.items.length > 0 && (
            <>
              <ol className="-mx-2 flex flex-col">
                {top.items.map((item, i) => (
                  <ItemRow
                    key={item.id}
                    item={item}
                    scanId={summary.scan}
                    total={root.allocated}
                    max={max}
                    accent={i === 0}
                  />
                ))}
                {top.total > top.items.length && (
                  <OthersRow
                    count={top.total - top.items.length}
                    bytes={root.allocated - top.items.reduce((a, i) => a + i.allocated, 0)}
                    total={root.allocated}
                    depth={0}
                  />
                )}
              </ol>
              <div aria-hidden className="mt-3 rule-total" />
              <div className={`${LINE} pt-4`}>
                <span className="col-span-full pl-5 sm:col-span-1">
                  <LeaderRow strong figure name={t('summary.sum')} value={bytes(root.allocated)} />
                </span>
              </div>
            </>
          )}
        </div>
      </article>

      <aside className="mt-8 flex flex-col gap-6 min-[1100px]:pt-6">
        {(notes.length > 0 || issueCount > 0) && (
          <div>
            <h2 className="text-secondary font-medium text-ink-2">{t('summary.notCounted')}</h2>
            <ul className="mt-2 flex flex-col gap-2 text-secondary text-ink-2">
              {notes.map((note) => (
                <li key={note}>{note}</li>
              ))}
              {issueCount > 0 && (
                <li>
                  <span>{t('summary.unreadable', count(issueCount))}</span>
                  {noAccess && (
                    <p className="mt-1">
                      {t('summary.noAccess')}{' '}
                      <button
                        type="button"
                        onClick={() => void openDiskAccessSettings().catch(() => undefined)}
                        className="rounded-tag text-ink underline underline-offset-2 hover:text-ink-2"
                      >
                        {t('access.open')}
                      </button>
                    </p>
                  )}
                  {issues.length > 0 && (
                    <details className="mt-1">
                      <summary className="flex min-h-6 w-fit cursor-pointer items-center rounded-tag text-ink-2 hover:text-ink">
                        {t('summary.unreadableShow', count(issues.length))}
                      </summary>
                      <ul className="mt-2 max-h-48 overflow-auto rounded-control bg-raised p-3 font-mono text-meta">
                        {issues.map((issue, i) => (
                          <li key={i} className="break-all" title={issue.error}>
                            {issue.path}
                          </li>
                        ))}
                      </ul>
                    </details>
                  )}
                </li>
              )}
            </ul>
          </div>
        )}
        <p className="text-meta text-ink-3 tabular-nums">
          {t('summary.stats', {
            files: formatCount(stats.files, locale),
            dirs: formatCount(stats.dirs, locale),
            elapsed: formatDuration(stats.elapsedMs, locale),
          })}
        </p>
        <div className="flex items-center gap-3">
          {root.hasChildren && (
            <Button variant="primary" onClick={() => browse(root)}>
              {t('summary.browseAll')}
            </Button>
          )}
          <button
            type="button"
            onClick={() => void scan(target)}
            className="ml-auto h-8 rounded-control px-2 text-body text-ink-2 transition-[background-color] duration-150 ease-out hover:bg-raised hover:text-ink"
          >
            {t('retry')}
          </button>
        </div>
      </aside>
    </section>
  )
}

type Children =
  | { status: 'loading' }
  | { status: 'ready'; items: NodeEntry[]; total: number }
  | { status: 'error' }

/** A folder's largest children, fetched while `open`. */
function useChildren(scanId: ScanId, id: number, open: boolean): Children | null {
  const [children, setChildren] = useState<{ id: number; value: Children } | null>(null)
  // Read again when disk changes are patched in; the old list shows meanwhile.
  const revision = useStore((s) => (s.last?.summary.scan === scanId ? s.last.revision : 0))
  useEffect(() => {
    if (!open) return
    let live = true
    getChildren(scanId, id, 'allocated', 'desc', 0, CHILD_ITEMS).then(
      (page) => live && setChildren({ id, value: { status: 'ready', items: page.items, total: page.total } }),
      () => live && setChildren({ id, value: { status: 'error' } }),
    )
    return () => {
      live = false
    }
  }, [scanId, id, open, revision])
  if (!open) return null
  return children?.id === id ? children.value : { status: 'loading' }
}

/**
 * One döküm line: `name ···· size`, a bar against the largest item, its
 * share of the total. A top-level folder opens one level in place when the
 * line is pressed; "Aç" goes into it in the table.
 */
function ItemRow({
  item,
  scanId,
  total,
  max,
  accent = false,
  depth = 0,
}: {
  item: NodeEntry
  scanId: ScanId
  total: number
  max: number
  accent?: boolean
  depth?: number
}) {
  const { t, locale } = useT()
  const browse = useStore((s) => s.browse)
  const opened = useStore((s) => s.opened)
  const toggleOpened = useStore((s) => s.toggleOpened)
  const isFolder = item.kind === 'dir' && item.hasChildren
  const canExpand = depth === 0 && isFolder
  const open = canExpand && opened.includes(item.id)
  const children = useChildren(scanId, item.id, open)
  const bar = total > 0 && item.allocated / total >= MIN_BAR_SHARE

  const cells = (
    <>
      <span className={`col-start-1 row-start-1 flex min-w-0 items-baseline gap-1 ${depth > 0 ? 'pl-10' : canExpand ? '' : 'pl-5'}`}>
        {canExpand && (
          <span aria-hidden className="shrink-0 self-center text-ink-3">
            <span
              className={`block transition-transform duration-150 ease-out motion-reduce:transition-none ${open ? 'rotate-90' : ''}`}
            >
              <ChevronIcon />
            </span>
          </span>
        )}
        <LeaderRow
          name={truncateMiddle(item.name, NAME_CHARS)}
          title={item.name}
          value={formatBytes(item.allocated, locale)}
        />
      </span>
      <span
        className={`col-start-1 row-start-2 self-center sm:col-start-2 sm:row-start-1 ${depth > 0 ? 'pl-10' : 'pl-5'} sm:pl-0`}
      >
        {bar && <ShareBar ratio={max > 0 ? item.allocated / max : 0} accent={accent} />}
      </span>
      <span className={`${SHARE_CELL} text-secondary text-ink-2 tabular-nums`}>
        {formatShare(item.allocated, total, locale)}
      </span>
    </>
  )

  const line = `${LINE} min-h-7 rounded-control px-2 py-1 ${depth > 0 ? 'text-secondary' : ''}`

  return (
    <li className="group relative">
      {canExpand ? (
        <div className="relative">
          <button
            type="button"
            aria-expanded={open}
            onClick={() => toggleOpened(item.id)}
            className={`${line} w-full text-left transition-[background-color] duration-150 ease-out hover:bg-raised`}
          >
            {cells}
          </button>
          <OpenLink item={item} onOpen={() => browse(item)} />
        </div>
      ) : (
        <div className={`${line} relative`}>
          {cells}
          {isFolder && <OpenLink item={item} onOpen={() => browse(item)} />}
        </div>
      )}

      {children?.status === 'loading' && (
        <p role="status" className="py-1 pl-12 text-secondary text-ink-2">
          {t('summary.loadingItems')}
        </p>
      )}
      {children?.status === 'error' && (
        <p role="status" className="py-1 pl-12 text-secondary text-ink-2">
          {t('summary.childrenError')}
        </p>
      )}
      {children?.status === 'ready' && (
        <ol className="flex flex-col pb-1">
          {children.items.map((child) => (
            <ItemRow key={child.id} item={child} scanId={scanId} total={total} max={max} depth={depth + 1} />
          ))}
          {children.total > children.items.length && (
            <OthersRow
              count={children.total - children.items.length}
              bytes={item.allocated - children.items.reduce((a, c) => a + c.allocated, 0)}
              total={total}
              depth={depth + 1}
            />
          )}
        </ol>
      )}
    </li>
  )
}

/** "Aç →": goes into a folder in the table. Shown on hover or focus, in the line's last column. */
function OpenLink({ item, onOpen }: { item: NodeEntry; onOpen: () => void }) {
  const { t } = useT()
  return (
    <button
      type="button"
      onClick={onOpen}
      aria-label={t('summary.openLabel', { name: item.name })}
      className="absolute top-1/2 right-2 flex min-h-6 w-10 -translate-y-1/2 sm:w-12 items-center justify-end gap-0.5 rounded-tag text-secondary text-ink-2 opacity-0 transition-opacity duration-150 ease-out group-hover:opacity-100 hover:text-ink focus-visible:opacity-100 motion-reduce:transition-none"
    >
      {t('summary.open')}
      <ChevronIcon />
    </button>
  )
}

/** The remainder after the listed items: `Diğer 214 öğe ···· 3,1 GB`. */
function OthersRow({ count, bytes, total, depth }: { count: number; bytes: number; total: number; depth: number }) {
  const { t, locale } = useT()
  const size = Math.max(0, bytes)
  return (
    <li className={`${LINE} min-h-7 px-2 py-1 text-ink-2 ${depth > 0 ? 'text-secondary' : ''}`}>
      <span className={`col-start-1 row-start-1 ${depth > 0 ? 'pl-10' : 'pl-5'}`}>
        <LeaderRow name={t('summary.others', { n: formatCount(count, locale), count })} value={formatBytes(size, locale)} />
      </span>
      <span className={`${SHARE_CELL} text-secondary tabular-nums`}>{formatShare(size, total, locale)}</span>
    </li>
  )
}
