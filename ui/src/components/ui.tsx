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

export const ChevronIcon = () => (
  <svg {...iconProps}>
    <path d="M6 3.5 10.5 8 6 12.5" />
  </svg>
)

export const CheckIcon = () => (
  <svg {...iconProps}>
    <path d="M3.5 8.5 6.5 11.5 12.5 4.5" />
  </svg>
)

/**
 * The app icon as the macOS Dock shows it (`assets/icon-macos.svg` without
 * the margin): brand colors in both themes.
 */
export const AppIcon = ({ size = 28 }: { size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 1024 1024" aria-hidden>
    <rect width="1024" height="1024" rx="230.4" fill="#2547b8" />
    <g fill="#f7f4ee" transform="translate(51.6 60.4) scale(.882)">
      <path d="M735.6 226.9A348 348 0 1 0 735.6 797.1L657.6 685.7A212 212 0 1 1 657.6 338.3Z" />
      <circle cx="779" cy="372" r="40" />
      <circle cx="816" cy="512" r="40" />
      <circle cx="779" cy="652" r="40" />
    </g>
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

/**
 * Signature: `name ········ size`, the leader sitting on the baseline.
 * `strong` is the subtotal line; `figure` sets the value as the screen's key
 * figure (28/500), as the döküm total does.
 */
export function LeaderRow({
  name,
  value,
  strong = false,
  figure = false,
  title,
}: {
  name: ReactNode
  value: ReactNode
  strong?: boolean
  figure?: boolean
  title?: string
}) {
  return (
    // Spans only, so a row can sit inside a button.
    <span className={`flex w-full min-w-0 items-baseline gap-2 ${strong ? 'text-strong font-medium' : ''}`}>
      <span className="min-w-0 truncate" title={title}>
        <bdi>{name}</bdi>
      </span>
      <span className="leader" aria-hidden />
      <span className={`shrink-0 font-mono tabular-nums ${figure ? 'text-figure font-medium' : ''}`}>{value}</span>
    </span>
  )
}

/**
 * Fullness bar, 12 tall. `used` fills the track; `parts` (largest items
 * first) split the filled part, 2px apart so each reads as its own block.
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
      className="flex h-3 w-full gap-0.5 overflow-hidden rounded-tag bg-raised"
    >
      {parts.map((bytes, i) => (
        <span key={i} className="h-full shrink-0 bg-ink-2" style={{ width: `${pct(bytes)}%` }} />
      ))}
      <span
        className={`h-full shrink-0 ${parts.length ? 'bg-ink-3' : 'bg-ink-2'}`}
        style={{ width: `${pct(parts.length ? rest : used)}%` }}
      />
    </div>
  )
}

/**
 * An item's size against the largest item: a thin bar in its own column,
 * scaled with `transform`, so it never animates layout. Decorative; the share
 * column carries the number.
 */
export function ShareBar({ ratio, accent = false }: { ratio: number; accent?: boolean }) {
  return (
    <span aria-hidden className="block h-1 w-full overflow-hidden rounded-full bg-raised">
      <span
        className={`block h-full origin-left rounded-full ${accent ? 'bg-accent' : 'bg-ink-3'}`}
        style={{ transform: `scaleX(${Math.min(1, Math.max(0, ratio))})` }}
      />
    </span>
  )
}

/** Stands in for the name when a translated sentence is split around it. */
export const NAME = '\u0000'

/**
 * A translated line with a name in it, `text` holding `NAME` where the name
 * goes: only the name truncates, the words around it always show.
 */
export function Named({ text, name }: { text: string; name: string }) {
  const [before = '', after = ''] = text.split(NAME)
  return (
    <span className="flex min-w-0 items-baseline whitespace-pre">
      {before && <span className="shrink-0">{before}</span>}
      <span className="min-w-0 truncate" title={name}>
        <bdi>{name}</bdi>
      </span>
      {after && <span className="shrink-0">{after}</span>}
    </span>
  )
}
