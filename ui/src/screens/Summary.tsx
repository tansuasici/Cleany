import { useState } from 'react'
import { Button, ChevronIcon, LeaderRow, ShareBar } from '../components/ui'
import { formatBytes, formatCount, formatDuration, formatShare } from '../lib/format'
import { type NodeEntry, type ScanId, getChildren } from '../lib/ipc'
import { type DoneScreen, useStore } from '../store'
import { useT } from '../useT'

type Props = { screen: DoneScreen }

/** How many children an opened folder lists in place. */
const CHILD_ITEMS = 8

/** Moment 2: the end-of-scan döküm — items with their share, one subtotal, footnotes. */
export function SummaryScreen({ screen }: Props) {
  const { t, locale } = useT()
  const scan = useStore((s) => s.scan)
  const showVolumes = useStore((s) => s.showVolumes)
  const browse = useStore((s) => s.browse)
  const { target, summary, top } = screen
  const { root, stats, issues, issueCount } = summary
  const bytes = (n: number) => formatBytes(n, locale)
  const count = (n: number) => ({ n: formatCount(n, locale), count: n })

  const notes = [
    stats.hardlinks > 0 && t('summary.hardlinks', count(stats.hardlinks)),
    stats.placeholders > 0 && t('summary.placeholders', count(stats.placeholders)),
    stats.skippedMounts > 0 && t('summary.skippedMounts', count(stats.skippedMounts)),
  ].filter((note): note is string => typeof note === 'string')

  const counted = bytes(root.allocated)
  const max = top.status === 'ready' ? (top.items[0]?.allocated ?? 0) : 0

  return (
    <section aria-labelledby="summary-title" className="mx-auto w-full max-w-200 px-8 py-10">
      <h1 id="summary-title" className="truncate text-title font-medium" title={target.name}>
        {t('summary.title', { name: target.name })}
      </h1>
      <p className="mt-1 text-ink-2 tabular-nums">
        {target.free === undefined
          ? t('summary.lede', { counted })
          : t('summary.ledeFree', { counted, free: bytes(target.free) })}
      </p>

      <article className="mt-8 rounded-panel border border-hairline bg-card p-6">
        <div className="grid grid-cols-[1fr_3.5rem] gap-x-4 text-secondary text-ink-2">
          <h2>{t('summary.items')}</h2>
          <span className="text-right">{t('summary.share')}</span>
        </div>
        <div className="mt-4" aria-live="polite">
          {top.status === 'loading' && <p className="text-ink-2">{t('summary.loadingItems')}</p>}
          {top.status === 'error' && <p className="text-ink-2">{t('summary.itemsError')}</p>}
          {top.status === 'ready' && top.items.length === 0 && (
            <p className="text-ink-2">{t('summary.noItems')}</p>
          )}
          {top.status === 'ready' && top.items.length > 0 && (
            <>
              <ol className="flex flex-col gap-3">
                {top.items.map((item, i) => (
                  <ItemRow
                    key={item.id}
                    item={item}
                    scanId={summary.scan}
                    total={root.allocated}
                    max={max}
                    accent={i === 0}
                    expandable
                  />
                ))}
                {top.total > top.items.length && (
                  <OthersRow
                    count={top.total - top.items.length}
                    bytes={root.allocated - top.items.reduce((a, i) => a + i.allocated, 0)}
                    total={root.allocated}
                    indent
                  />
                )}
              </ol>
              <div className="mt-4 grid grid-cols-[1fr_3.5rem] gap-x-4 border-t border-ink pt-3">
                <span className="pl-5">
                  <LeaderRow strong name={t('summary.sum')} value={counted} />
                </span>
              </div>
            </>
          )}
        </div>

        {(notes.length > 0 || issueCount > 0) && (
          <>
            <h2 className="mt-8 text-secondary text-ink-2">{t('summary.notCounted')}</h2>
            <ul className="mt-2 flex flex-col gap-1 text-secondary text-ink-2">
              {notes.map((note) => (
                <li key={note} className="flex gap-2">
                  <span aria-hidden>*</span>
                  <span>{note}</span>
                </li>
              ))}
              {issueCount > 0 && (
                <li className="flex gap-2">
                  <span aria-hidden>*</span>
                  <div className="min-w-0 flex-1">
                    <span>{t('summary.unreadable', count(issueCount))}</span>
                    {issues.length > 0 && (
                      <details className="mt-1">
                        <summary className="w-fit cursor-pointer rounded-tag text-ink-2 hover:text-ink">
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
                  </div>
                </li>
              )}
            </ul>
          </>
        )}
        <p className="mt-6 text-meta text-ink-3 tabular-nums">
          {t('summary.stats', {
            files: formatCount(stats.files, locale),
            dirs: formatCount(stats.dirs, locale),
            elapsed: formatDuration(stats.elapsedMs, locale),
          })}
        </p>
      </article>

      <div className="mt-6 flex items-center gap-3">
        {root.hasChildren && <Button onClick={() => browse(root)}>{t('summary.browseAll')}</Button>}
        <Button onClick={showVolumes}>{t('backToVolumes')}</Button>
        <button
          type="button"
          onClick={() => void scan(target)}
          className="h-8 rounded-control px-2 text-body text-ink-2 transition-[background-color] duration-150 ease-out hover:bg-raised hover:text-ink"
        >
          {t('retry')}
        </button>
      </div>
    </section>
  )
}

type Children =
  | { status: 'closed' }
  | { status: 'loading' }
  | { status: 'ready'; items: NodeEntry[]; total: number }
  | { status: 'error' }

/**
 * One döküm line: `name ···· size`, its share of the total, and a bar against
 * the largest item. A top-level folder opens one level in place.
 */
function ItemRow({
  item,
  scanId,
  total,
  max,
  accent = false,
  expandable = false,
}: {
  item: NodeEntry
  scanId: ScanId
  total: number
  max: number
  accent?: boolean
  expandable?: boolean
}) {
  const { t, locale } = useT()
  const browse = useStore((s) => s.browse)
  const [children, setChildren] = useState<Children>({ status: 'closed' })
  const isFolder = item.kind === 'dir' && item.hasChildren
  const canExpand = expandable && isFolder
  const open = children.status !== 'closed'
  const row = <LeaderRow name={item.name} title={item.name} value={formatBytes(item.allocated, locale)} />

  const toggle = async () => {
    if (open) {
      setChildren({ status: 'closed' })
      return
    }
    setChildren({ status: 'loading' })
    try {
      const page = await getChildren(scanId, item.id, 'allocated', 'desc', 0, CHILD_ITEMS)
      setChildren({ status: 'ready', items: page.items, total: page.total })
    } catch {
      setChildren({ status: 'error' })
    }
  }

  return (
    <li>
      <div className="grid grid-cols-[1fr_3.5rem] items-baseline gap-x-4 gap-y-1.5">
        <span className={`flex min-w-0 items-baseline gap-1 ${expandable && !canExpand ? 'pl-5' : ''}`}>
          {canExpand && (
            // The chevron opens a level in place; the name goes into the folder.
            <button
              type="button"
              aria-expanded={open}
              aria-label={t('summary.expandLabel', { name: item.name })}
              onClick={() => void toggle()}
              className="-m-1 shrink-0 self-center rounded-tag p-1 text-ink-3 hover:bg-raised hover:text-ink"
            >
              <span
                className={`block transition-transform duration-150 ease-out motion-reduce:transition-none ${open ? 'rotate-90' : ''}`}
              >
                <ChevronIcon />
              </span>
            </button>
          )}
          {isFolder ? (
            <button
              type="button"
              onClick={() => browse(item)}
              className="-mx-1 flex min-w-0 flex-1 rounded-tag px-1 text-left hover:bg-raised"
            >
              {row}
            </button>
          ) : (
            row
          )}
        </span>
        <span className="text-right font-mono text-secondary text-ink-2 tabular-nums">
          {formatShare(item.allocated, total, locale)}
        </span>
        <span className={expandable ? 'pl-5' : ''}>
          <ShareBar ratio={max > 0 ? item.allocated / max : 0} accent={accent} />
        </span>
      </div>

      {children.status === 'loading' && (
        <p className="mt-2 pl-8 text-secondary text-ink-2">{t('summary.loadingItems')}</p>
      )}
      {children.status === 'error' && (
        <p className="mt-2 pl-8 text-secondary text-ink-2">{t('summary.childrenError')}</p>
      )}
      {children.status === 'ready' && (
        <ol className="mt-3 ml-2 flex flex-col gap-3 border-l border-hairline pl-6 text-secondary">
          {children.items.map((child) => (
            <ItemRow key={child.id} item={child} scanId={scanId} total={total} max={max} />
          ))}
          {children.total > children.items.length && (
            <OthersRow
              count={children.total - children.items.length}
              bytes={item.allocated - children.items.reduce((a, c) => a + c.allocated, 0)}
              total={total}
            />
          )}
        </ol>
      )}
    </li>
  )
}

/** The remainder after the listed items: `Diğer 214 öğe ···· 3,1 GB`. */
function OthersRow({
  count,
  bytes,
  total,
  indent = false,
}: {
  count: number
  bytes: number
  total: number
  indent?: boolean
}) {
  const { t, locale } = useT()
  const size = Math.max(0, bytes)
  return (
    <li className="grid grid-cols-[1fr_3.5rem] items-baseline gap-x-4 text-ink-2">
      <span className={indent ? 'pl-5' : ''}>
        <LeaderRow
          name={t('summary.others', { n: formatCount(count, locale), count })}
          value={formatBytes(size, locale)}
        />
      </span>
      <span className="text-right font-mono text-secondary tabular-nums">{formatShare(size, total, locale)}</span>
    </li>
  )
}
