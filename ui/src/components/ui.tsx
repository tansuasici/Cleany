import type { ButtonHTMLAttributes, ReactNode } from 'react'

// Shared pieces of the döküm direction (ui/DESIGN_MAP.md). Icons are 16px,
// 1.5 stroke, drawn inline; no icon set.

const iconProps = {
  width: 16,
  height: 16,
  viewBox: '0 0 16 16',
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 1.5,
  strokeLinecap: 'round' as const,
  strokeLinejoin: 'round' as const,
  'aria-hidden': true,
}

export const BackIcon = () => (
  <svg {...iconProps}>
    <path d="M10 3.5 5.5 8l4.5 4.5" />
  </svg>
)

export const ChevronIcon = () => (
  <svg {...iconProps}>
    <path d="M6 3.5 10.5 8 6 12.5" />
  </svg>
)

export const VolumeIcon = () => (
  <svg {...iconProps}>
    <rect x="1.75" y="4.25" width="12.5" height="7.5" rx="1.5" />
    <path d="M4.5 9h3" />
    <circle cx="11.5" cy="9" r=".5" fill="currentColor" stroke="none" />
  </svg>
)

export const RemovableIcon = () => (
  <svg {...iconProps}>
    <path d="M8 2.5 3.5 8h9L8 2.5Z" />
    <path d="M3.5 11.5h9" />
  </svg>
)

type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & { variant?: 'primary' | 'secondary' }

/** Row/control radius 6; primary uses the accent, everything else stays ink. */
export function Button({ variant = 'secondary', className = '', ...props }: ButtonProps) {
  const look =
    variant === 'primary'
      ? 'bg-accent text-on-accent hover:opacity-90'
      : 'border border-hairline bg-card text-ink hover:bg-raised'
  return (
    <button
      type="button"
      className={`inline-flex h-8 items-center gap-2 rounded-control px-3 text-body font-medium transition-[background-color,opacity] duration-150 ease-out disabled:opacity-50 ${look} ${className}`}
      {...props}
    />
  )
}

/** Signature: `name ········ size`, the leader sitting on the baseline. */
export function LeaderRow({
  name,
  value,
  strong = false,
  title,
}: {
  name: ReactNode
  value: ReactNode
  strong?: boolean
  title?: string
}) {
  return (
    // Spans only, so a row can sit inside a button.
    <span className={`flex w-full min-w-0 items-baseline gap-2 ${strong ? 'text-strong font-medium' : ''}`}>
      <span className="min-w-0 truncate" title={title}>
        {name}
      </span>
      <span className="leader" aria-hidden />
      <span className="shrink-0 font-mono tabular-nums">{value}</span>
    </span>
  )
}

/**
 * Fullness bar. `used` fills the track; `parts` (largest items first) split
 * the filled part, each separated by a card-coloured gap.
 */
export function UsageBar({
  total,
  used,
  parts = [],
  label,
}: {
  total: number
  used: number
  parts?: number[]
  label: string
}) {
  const pct = (bytes: number) => (total > 0 ? Math.min(100, (bytes / total) * 100) : 0)
  const partsSum = parts.reduce((a, b) => a + b, 0)
  const rest = Math.max(0, used - partsSum)
  return (
    <div
      role="img"
      aria-label={label}
      className="flex h-1.5 w-full overflow-hidden rounded-full bg-raised"
    >
      {parts.map((bytes, i) => (
        <span
          key={i}
          className="h-full border-r border-card bg-ink-2"
          style={{ width: `${pct(bytes)}%` }}
        />
      ))}
      <span
        className={`h-full ${parts.length ? 'bg-ink-3' : 'bg-ink-2'}`}
        style={{ width: `${pct(parts.length ? rest : used)}%` }}
      />
    </div>
  )
}

/**
 * An item's size against the largest item: a thin bar scaled with
 * `transform`, so it never animates layout. Decorative; the share column
 * carries the number.
 */
export function ShareBar({ ratio, accent = false }: { ratio: number; accent?: boolean }) {
  return (
    <span aria-hidden className="block h-1 w-full overflow-hidden rounded-full">
      <span
        className={`block h-full origin-left rounded-full ${accent ? 'bg-accent' : 'bg-ink-3'}`}
        style={{ transform: `scaleX(${Math.min(1, Math.max(0, ratio))})` }}
      />
    </span>
  )
}
