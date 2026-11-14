import { useEffect, useState } from 'react'
import { type NodeEntry, type ScanId, getAncestors } from './lib/ipc'

/** Root-first ancestors of `node`, itself last; `null` until they arrive. */
export function useTrail(scan: ScanId, node: NodeEntry): NodeEntry[] | null {
  const [trail, setTrail] = useState<{ id: number; list: NodeEntry[] } | null>(null)
  useEffect(() => {
    let live = true
    getAncestors(scan, node.id).then(
      (list) => live && setTrail({ id: node.id, list }),
      // Without the chain the table still works; the breadcrumb shows only the folder itself.
      () => live && setTrail({ id: node.id, list: [node] }),
    )
    return () => {
      live = false
    }
  }, [scan, node])
  return trail?.id === node.id ? trail.list : null
}
