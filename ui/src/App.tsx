import { useEffect } from 'react'
import { ChevronIcon } from './components/ui'
import type { NodeEntry } from './lib/ipc'
import { BrowseView } from './screens/Browse'
import { EndedScreen } from './screens/Ended'
import { ScanningScreen } from './screens/Scanning'
import { SummaryScreen } from './screens/Summary'
import { VolumesScreen } from './screens/Volumes'
import { type BrowseScreen, type Screen, useStore } from './store'
import { useT } from './useT'
import { useTrail } from './useTrail'

export default function App() {
  const { t, locale } = useT()
  const screen = useStore((s) => s.screen)
  const setLocale = useStore((s) => s.setLocale)
  const loadVolumes = useStore((s) => s.loadVolumes)

  useEffect(() => {
    void loadVolumes()
  }, [loadVolumes])

  useEffect(() => {
    document.documentElement.lang = locale
  }, [locale])

  return (
    <div className="flex h-full flex-col">
      <header className="flex h-12 shrink-0 items-center gap-3 border-b border-hairline bg-card px-4">
        <Trail screen={screen} />
        <button
          type="button"
          lang={locale === 'tr' ? 'en' : 'tr'}
          aria-label={t('app.switchLocaleLabel')}
          onClick={() => setLocale(locale === 'tr' ? 'en' : 'tr')}
          className="ml-auto inline-flex h-8 items-center rounded-control px-2 text-secondary text-ink-2 transition-[background-color] duration-150 ease-out hover:bg-raised hover:text-ink"
        >
          {t('app.switchLocale')}
        </button>
      </header>

      <main className="min-h-0 flex-1 overflow-auto [scrollbar-gutter:stable]">
        {screen.kind === 'volumes' && <VolumesScreen />}
        {screen.kind === 'scanning' && <ScanningScreen screen={screen} />}
        {screen.kind === 'done' && <SummaryScreen screen={screen} />}
        {screen.kind === 'browse' && <BrowseView screen={screen} />}
        {(screen.kind === 'cancelled' || screen.kind === 'failed') && <EndedScreen screen={screen} />}
      </main>
    </div>
  )
}

type Crumb = { key: string | number; label: string; onPress?: () => void }

/**
 * Where the user is, as one line: `Birimler / CleanyTree dökümü / Tümü / Projeler`.
 * Every step but the last goes back to it; the last is the current screen.
 */
function Trail({ screen }: { screen: Screen }) {
  const { t } = useT()
  const showVolumes = useStore((s) => s.showVolumes)
  if (screen.kind === 'volumes') return <span className="text-strong font-medium">{t('app.name')}</span>
  if (screen.kind === 'browse') return <BrowseTrail screen={screen} />

  const volumes: Crumb = { key: 'volumes', label: t('app.volumes'), onPress: showVolumes }
  const here =
    screen.kind === 'scanning'
      ? t('scan.title', { name: screen.target.name })
      : screen.kind === 'done'
        ? t('summary.title', { name: screen.target.name })
        : t(screen.kind === 'failed' ? 'failed.title' : 'cancelled.title')
  return <Crumbs crumbs={[volumes, { key: 'here', label: here }]} />
}

function BrowseTrail({ screen }: { screen: BrowseScreen }) {
  const { t } = useT()
  const showVolumes = useStore((s) => s.showVolumes)
  const showSummary = useStore((s) => s.showSummary)
  const browse = useStore((s) => s.browse)
  const { done, node } = screen
  const root = done.summary.root.id
  // Until the chain arrives, the current folder stands alone after the root.
  const trail = useTrail(done.summary.scan, node) ?? (node.id === root ? [node] : [done.summary.root, node])
  const step = (n: NodeEntry): Crumb => ({
    key: n.id,
    label: n.id === root ? t('browse.all') : n.name,
    onPress: () => browse(n),
  })
  return (
    <Crumbs
      crumbs={[
        { key: 'volumes', label: t('app.volumes'), onPress: showVolumes },
        { key: 'summary', label: t('summary.title', { name: done.target.name }), onPress: showSummary },
        ...trail.map(step),
      ]}
    />
  )
}

function Crumbs({ crumbs }: { crumbs: Crumb[] }) {
  const { t } = useT()
  return (
    <nav aria-label={t('app.trail')} className="min-w-0">
      <ol className="flex min-w-0 items-center gap-x-0.5 text-body text-ink-2">
        {crumbs.map((crumb, i) => {
          const last = i === crumbs.length - 1
          return (
            <li key={crumb.key} className={`flex items-center gap-x-0.5 ${last ? 'min-w-0' : 'min-w-0 shrink-[2]'}`}>
              {last ? (
                <bdi aria-current="page" className="truncate px-1 font-medium text-ink" title={crumb.label}>
                  {crumb.label}
                </bdi>
              ) : (
                <>
                  <button
                    type="button"
                    onClick={crumb.onPress}
                    title={crumb.label}
                    className="flex min-h-6 min-w-0 items-center rounded-tag px-1 transition-[background-color] duration-150 ease-out hover:bg-raised hover:text-ink motion-reduce:transition-none"
                  >
                    <bdi className="truncate">{crumb.label}</bdi>
                  </button>
                  <span aria-hidden className="shrink-0 text-ink-3">
                    <ChevronIcon />
                  </span>
                </>
              )}
            </li>
          )
        })}
      </ol>
    </nav>
  )
}
