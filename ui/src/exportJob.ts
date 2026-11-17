import { create } from 'zustand'
import {
  type ExportError,
  type ExportFormat,
  type ExportReport,
  type ExportScope,
  type ScanId,
  cancelExport,
  exportScan,
} from './lib/ipc'

/**
 * The export on show. It lives outside the screens, so the progress and the
 * result stay visible while the user moves between folders and views.
 */
export type ExportJob =
  | { phase: 'running'; fileName: string; rows: number; total: number | null; stopping: boolean }
  | { phase: 'done'; report: ExportReport }
  | { phase: 'stopped' }
  | { phase: 'failed'; stale: boolean }

interface ExportState {
  job: ExportJob | null
  /** Opens the save dialog and writes `scope`. Ignored while an export runs. */
  start: (scan: ScanId, scope: ExportScope, format: ExportFormat) => Promise<void>
  /** Asks the running export to stop; it then ends as `stopped`. */
  stop: () => void
  dismiss: () => void
}

let busy = false

export const useExport = create<ExportState>((set, get) => ({
  job: null,
  start: async (scan, scope, format) => {
    if (busy) return
    busy = true
    // A finished notice gives way to the next export once its file is chosen.
    try {
      const report = await exportScan(scan, scope, format, (event) => {
        if (event.kind === 'started') {
          set({ job: { phase: 'running', fileName: event.fileName, rows: 0, total: null, stopping: false } })
          return
        }
        const job = get().job
        if (job?.phase === 'running') set({ job: { ...job, rows: event.rows, total: event.total } })
      })
      // A closed save dialog leaves the last notice as it was.
      if (report) set({ job: { phase: 'done', report } })
    } catch (e) {
      const kind = (e as ExportError | undefined)?.kind
      set({ job: kind === 'cancelled' ? { phase: 'stopped' } : { phase: 'failed', stale: kind === 'staleScan' } })
    } finally {
      busy = false
    }
  },
  stop: () => {
    const job = get().job
    if (job?.phase !== 'running' || job.stopping) return
    set({ job: { ...job, stopping: true } })
    void cancelExport()
  },
  dismiss: () => {
    if (get().job?.phase !== 'running') set({ job: null })
  },
}))
