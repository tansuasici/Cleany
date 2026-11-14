import { Button, RemovableIcon, UsageBar, VolumeIcon } from '../components/ui'
import { formatBytes } from '../lib/format'
import type { VolumeEntry } from '../lib/ipc'
import { useStore } from '../store'
import { useT } from '../useT'

/** The system volume: `/` on macOS and Linux, `C:\` on Windows. */
const isBoot = (mount: string) => mount === '/' || /^c:\\?$/i.test(mount)

/** Boot volume first, then the largest. */
function ordered(list: VolumeEntry[]): VolumeEntry[] {
  return [...list].sort(
    (a, b) => Number(isBoot(b.mount)) - Number(isBoot(a.mount)) || b.total - a.total,
  )
}

export function VolumesScreen() {
  const { t } = useT()
  const volumes = useStore((s) => s.volumes)
  const loadVolumes = useStore((s) => s.loadVolumes)
  const lastPath = useStore((s) => s.last?.target.path)

  return (
    <section aria-labelledby="volumes-title" className="mx-auto w-full max-w-200 px-8 py-10">
      <h1 id="volumes-title" className="text-title font-medium">
        {t('volumes.title')}
      </h1>
      <p className="mt-1 text-ink-2">{t('volumes.lede')}</p>

      <div className="mt-8" aria-live="polite">
        {volumes.status === 'loading' && <p className="text-ink-2">{t('volumes.loading')}</p>}
        {volumes.status === 'error' && (
          <div className="flex flex-col items-start gap-3">
            <p>{t('volumes.error')}</p>
            <p className="font-mono text-meta text-ink-3">{volumes.message}</p>
            <Button onClick={() => void loadVolumes()}>{t('volumes.retry')}</Button>
          </div>
        )}
        {volumes.status === 'ready' && volumes.list.length === 0 && (
          <p className="text-ink-2">{t('volumes.empty')}</p>
        )}
        {volumes.status === 'ready' && volumes.list.length > 0 && (
          <ul className="divide-y divide-hairline rounded-panel border border-hairline bg-card">
            {ordered(volumes.list).map((v, i) => (
              <li key={v.mount}>
                {/* One primary per screen: the open breakdown if there is one, else the first volume. */}
                <VolumeRow volume={v} primary={lastPath === undefined ? i === 0 : lastPath === v.mount} />
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  )
}

function VolumeRow({ volume, primary }: { volume: VolumeEntry; primary: boolean }) {
  const { t, locale } = useT()
  const scan = useStore((s) => s.scan)
  const openLast = useStore((s) => s.openLast)
  const hasLast = useStore((s) => s.last?.target.path === volume.mount)
  const breakdown = useStore((s) => s.breakdowns[volume.mount])
  const used = Math.max(0, volume.total - volume.available)
  const name = volume.name || volume.mount
  const bytes = (n: number) => formatBytes(n, locale)
  const usage = t('volumes.usage', { used: bytes(used), total: bytes(volume.total) })
  const start = () => void scan({ path: volume.mount, name, used, free: volume.available })

  return (
    <article aria-label={name} className="flex flex-col gap-4 p-6">
      <div className="flex items-start gap-3">
        <span className="mt-0.5 text-ink-2">
          <VolumeIcon />
        </span>
        <div className="min-w-0 flex-1">
          <p className="flex items-center gap-2">
            <span className="truncate text-strong font-medium" title={name}>
              {name}
            </span>
            {volume.removable && (
              <span className="inline-flex shrink-0 items-center gap-1 rounded-tag bg-raised px-1.5 py-0.5 text-meta text-ink-2">
                <RemovableIcon />
                {t('volumes.removable')}
              </span>
            )}
          </p>
          {volume.mount !== '/' && (
            <p className="truncate font-mono text-meta text-ink-3" title={volume.mount}>
              {volume.mount}
            </p>
          )}
        </div>
        <div className="flex shrink-0 gap-2">
          {hasLast ? (
            <>
              <Button aria-label={t('volumes.rescanLabel', { name })} onClick={start}>
                {t('volumes.rescan')}
              </Button>
              <Button
                variant={primary ? 'primary' : 'secondary'}
                aria-label={t('volumes.openLabel', { name })}
                onClick={openLast}
              >
                {t('volumes.open')}
              </Button>
            </>
          ) : (
            <Button
              variant={primary ? 'primary' : 'secondary'}
              aria-label={t('volumes.scanLabel', { name })}
              onClick={start}
            >
              {t('volumes.scan')}
            </Button>
          )}
        </div>
      </div>

      <div className="flex flex-col gap-2 pl-7">
        <p className="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
          <span className="flex items-baseline gap-2">
            <span className="text-figure font-medium tabular-nums">{bytes(volume.available)}</span>
            <span className="text-ink-2">{t('volumes.free')}</span>
          </span>
          <span className="text-secondary text-ink-2 tabular-nums">{usage}</span>
        </p>
        <UsageBar
          total={volume.total}
          used={used}
          parts={breakdown?.items.map((i) => i.allocated)}
          label={usage}
        />
        {breakdown && <p className="text-meta text-ink-3">{t('volumes.lastScan')}</p>}
      </div>
    </article>
  )
}
