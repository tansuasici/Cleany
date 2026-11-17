import { useEffect } from 'react'
import { SettingsDialog } from './components/SettingsDialog'
import { AppIcon, Button, ChevronIcon } from './components/ui'
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
  const showVolumes = useStore((s) => s.showVolumes)
  const loadVolumes = useStore((s) => s.loadVolumes)
  const checkAccess = useStore((s) => s.checkAccess)
  const reattach = useStore((s) => s.reattach)

  useEffect(() => {
    void loadVolumes()
  }, [loadVolumes])

  // After a webview reload the backend may still hold a finished scan.
  useEffect(() => {
    void reattach()
  }, [reattach])

  // Coming back from System Settings is the moment access may have changed.
  useEffect(() => {
    void checkAccess()
    const onFocus = () => void checkAccess()
    window.addEventListener('focus', onFocus)
    return () => window.removeEventListener('focus', onFocus)
  }, [checkAccess])

  useEffect(() => {
    document.documentElement.lang = locale
  }, [locale])

  return (
    <div className="flex h-full flex-col">
      <header className="flex h-12 shrink-0 items-center gap-3 bg-page px-4 max-sm:gap-2 max-sm:px-3">
        <button
          type="button"
          onClick={showVolumes}
          aria-label={t('app.home')}
          className="inline-flex h-8 shrink-0 items-center gap-2 rounded-control px-1.5 text-ink transition-[background-color] duration-150 ease-out hover:bg-raised motion-reduce:transition-none"
        >
          <AppIcon />
          <span aria-hidden className="text-strong font-medium max-sm:hidden">
            Cleany
          </span>
        </button>
        <span aria-hidden className="h-5 w-px shrink-0 bg-hairline" />
        <Trail screen={screen} />
        <div className="ml-auto flex shrink-0 items-center">
          <SettingsDialog />
        </div>
      </header>

      <main className="flex min-h-0 flex-1 flex-col overflow-auto [scrollbar-gutter:stable]">
        <Outdated screen={screen} />
        {/* Keyed by kind: a new screen enters once, moving between folders does not. */}
        <div key={screen.kind} className="flex min-h-0 flex-1 flex-col enter-screen">
          {screen.kind === 'volumes' && <VolumesScreen />}
          {screen.kind === 'scanning' && <ScanningScreen screen={screen} />}
          {screen.kind === 'done' && <SummaryScreen screen={screen} />}
          {screen.kind === 'browse' && <BrowseView screen={screen} />}
          {(screen.kind === 'cancelled' || screen.kind === 'failed') && <EndedScreen screen={screen} />}
        </div>
      </main>
    </div>
  )
}

/** Shown once live updates gave up on the finished scan on show: too much changed on disk. */
function Outdated({ screen }: { screen: Screen }) {
  const { t } = useT()
  const scan = useStore((s) => s.scan)
  const done = screen.kind === 'done' ? screen : screen.kind === 'browse' ? screen.done : null
  if (!done?.outdated) return null
  return (
    <div
      role="status"
      className="mx-4 mt-4 flex shrink-0 items-center gap-3 rounded-card bg-raised py-2 pr-2 pl-4 enter-fade max-sm:mx-3"
    >
      <p className="min-w-0 flex-1 text-secondary text-ink-2">{t('live.outdated')}</p>
      <Button onClick={() => void scan(done.target)}>{t('retry')}</Button>
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
  if (screen.kind === 'volumes') return <Crumbs crumbs={[{ key: 'volumes', label: t('app.volumes') }]} />
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
