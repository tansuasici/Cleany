import { useId, useState } from 'react'
import { Button, ChevronIcon, LeaderRow, Size } from '../components/ui'
import { formatBytes, formatCount } from '../lib/format'
import type { Basket } from '../store'
import { useT } from '../useT'

/**
 * The basket under the actions: how many items, the space they give back,
 * and the move that takes them all. Shown only while the basket holds
 * something; the list opens in place.
 */
export function BasketBar({
  basket,
  onTrash,
  onDelete,
  onRemove,
  onClear,
}: {
  basket: Basket
  onTrash: () => void
  /** Set while Settings allow permanent delete. */
  onDelete?: () => void
  onRemove: (id: number) => void
  onClear: () => void
}) {
  const { t, locale } = useT()
  const [open, setOpen] = useState(false)
  const listId = useId()
  const { items } = basket
  const count = items.length
  const total = items.reduce((sum, item) => sum + item.allocated, 0)

  return (
    <section aria-label={t('basket.label')} className="rounded-panel border border-card-edge bg-card px-4 py-3 shadow-card enter-fade">
      <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
        <p className="flex min-w-0 flex-1 flex-wrap items-baseline gap-x-2 text-body text-ink-2">
          <span className="font-medium text-ink">{t('basket.title')}</span>
          <span aria-hidden>·</span>
          <span className="tabular-nums">{t('basket.count', { n: formatCount(count, locale), count })}</span>
          <span aria-hidden>·</span>
          <span>
            {t('basket.space')} <Size value={formatBytes(total, locale)} className="text-ink" />
          </span>
        </p>
        <div className="flex flex-wrap gap-2">
          <Button aria-expanded={open} aria-controls={listId} onClick={() => setOpen((o) => !o)}>
            {t(open ? 'basket.hideList' : 'basket.showList')}
            {/* Turns at once: it answers a press. */}
            <span aria-hidden className={`inline-flex ${open ? 'rotate-90' : ''}`}>
              <ChevronIcon />
            </span>
          </Button>
          <Button onClick={onClear}>{t('basket.clear')}</Button>
          {onDelete && (
            <Button onClick={onDelete} aria-haspopup="dialog">
              {t('basket.delete')}
            </Button>
          )}
          <Button variant="primary" onClick={onTrash} aria-haspopup="dialog">
            {t('basket.trash')}
          </Button>
        </div>
      </div>
      <ul id={listId} hidden={!open} className="mt-3 flex flex-col border-t border-hairline pt-2">
        {items.map((item) => (
          <li key={item.id} className="flex h-8 items-center gap-3 text-body">
            <LeaderRow name={item.name} title={item.name} value={formatBytes(item.allocated, locale)} />
            <Button
              className="h-6 px-2 text-secondary"
              onClick={() => onRemove(item.id)}
              aria-label={t('basket.removeOne', { name: item.name })}
            >
              {t('basket.removeShort')}
            </Button>
          </li>
        ))}
      </ul>
    </section>
  )
}
