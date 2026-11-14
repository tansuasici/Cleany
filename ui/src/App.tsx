import { useEffect } from 'react'
import { BackIcon } from './components/ui'
import { EndedScreen } from './screens/Ended'
import { ScanningScreen } from './screens/Scanning'
import { SummaryScreen } from './screens/Summary'
import { VolumesScreen } from './screens/Volumes'
import { useStore } from './store'
import { useT } from './useT'

export default function App() {
  const { t, locale } = useT()
  const screen = useStore((s) => s.screen)
  const setLocale = useStore((s) => s.setLocale)
  const showVolumes = useStore((s) => s.showVolumes)
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
        {screen.kind !== 'volumes' ? (
          <button
            type="button"
            onClick={showVolumes}
            className="-ml-1 inline-flex h-8 items-center gap-1 rounded-control px-2 text-body text-ink-2 transition-[background-color] duration-150 ease-out hover:bg-raised hover:text-ink"
          >
            <BackIcon />
            {t('app.back')}
          </button>
        ) : (
          <span className="text-strong font-medium">{t('app.name')}</span>
        )}
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
        {(screen.kind === 'cancelled' || screen.kind === 'failed') && <EndedScreen screen={screen} />}
      </main>
    </div>
  )
}
