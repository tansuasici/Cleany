import { type ComponentProps, type KeyboardEvent, type PointerEvent, type ReactNode, useRef, useState } from 'react'

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
      <circle cx="779" cy="372" r="50" />
      <circle cx="816" cy="512" r="50" />
      <circle cx="779" cy="652" r="50" />
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

/** Show in the file manager: a folder with a dot of focus on it. */
export const RevealIcon = () => (
  <svg {...iconProps}>
    <path d="M1.75 4.25a1 1 0 0 1 1-1h3l1.5 1.5h6a1 1 0 0 1 1 1v6.5a1 1 0 0 1-1 1H2.75a1 1 0 0 1-1-1Z" />
    <circle cx="8" cy="9" r="1.5" />
  </svg>
)

/** Open in its own app: an arrow leaving its box. */
export const OpenIcon = () => (
  <svg {...iconProps}>
    <path d="M7 3H3.75a1 1 0 0 0-1 1v8.25a1 1 0 0 0 1 1H12a1 1 0 0 0 1-1V9" />
    <path d="M9.5 2.75h3.75V6.5M13 3 7.5 8.5" />
  </svg>
)

export const CopyIcon = () => (
  <svg {...iconProps}>
    <rect x="5.25" y="5.25" width="8.5" height="8.5" rx="1.5" />
    <path d="M10.75 5V3.75a1.5 1.5 0 0 0-1.5-1.5h-5.5a1.5 1.5 0 0 0-1.5 1.5v5.5a1.5 1.5 0 0 0 1.5 1.5H5" />
  </svg>
)

export const TerminalIcon = () => (
  <svg {...iconProps}>
    <rect x="2" y="2.75" width="12" height="10.5" rx="1.5" />
    <path d="m4.75 6.25 2 1.75-2 1.75M8.5 10h2.75" />
  </svg>
)

export const BasketIcon = () => (
  <svg {...iconProps}>
    <path d="M2 6.25h12l-1.25 6.5a1 1 0 0 1-1 .75h-7.5a1 1 0 0 1-1-.75Z" />
    <path d="M5.5 6.25 7 2.5M10.5 6.25 9 2.5" />
  </svg>
)

export const TrashIcon = () => (
  <svg {...iconProps}>
    <path d="M2.5 4.25h11M6.25 4.25V2.75h3.5v1.5" />
    <path d="M3.75 4.25 4.5 13a1 1 0 0 0 1 .75h5a1 1 0 0 0 1-.75l.75-8.75" />
  </svg>
)

export const GearIcon = () => (
  <svg {...iconProps}>
    <path d="M6.55 3.32 6.58 1.66 9.42 1.66 9.45 3.32 10.29 3.67 11.48 2.51 13.49 4.52 12.33 5.71 12.68 6.55 14.34 6.58 14.34 9.42 12.68 9.45 12.33 10.29 13.49 11.48 11.48 13.49 10.29 12.33 9.45 12.68 9.42 14.34 6.58 14.34 6.55 12.68 5.71 12.33 4.52 13.49 2.51 11.48 3.67 10.29 3.32 9.45 1.66 9.42 1.66 6.58 3.32 6.55 3.67 5.71 2.51 4.52 4.52 2.51 5.71 3.67Z" />
    <circle cx="8" cy="8" r="2" />
  </svg>
)

type ButtonProps = ComponentProps<'button'> & { variant?: 'primary' | 'secondary' }

/** Row/control radius 8; primary uses the accent, everything else stays ink. A press settles it to .97. */
export function Button({ variant = 'secondary', className = '', ...props }: ButtonProps) {
  const look =
    variant === 'primary'
      ? 'bg-accent text-on-accent hover:opacity-90'
      : 'border border-edge bg-card text-ink hover:bg-raised'
  return (
    <button
      type="button"
      className={`inline-flex h-8 shrink-0 items-center gap-2 rounded-control px-3 whitespace-nowrap text-body font-medium transition-[background-color,opacity,transform] duration-100 ease-out not-disabled:active:scale-[.97] disabled:opacity-50 motion-reduce:transition-[background-color,opacity] motion-reduce:active:scale-100 ${look} ${className}`}
      {...props}
    />
  )
}

/**
 * A formatted size (`48,2 GB`) as a key figure: the number in tabular figures, the unit
 * after it in small sans (DESIGN_MAP → Type scale), so the eye reads the number.
 */
export function Size({ value, className = '' }: { value: string; className?: string }) {
  const at = value.lastIndexOf(' ')
  if (at < 0) return <span className={`tabular-nums ${className}`}>{value}</span>
  return (
    <span className={`whitespace-nowrap ${className}`}>
      <span className="tabular-nums">{value.slice(0, at)}</span>
      <span className="unit">{value.slice(at)}</span>
    </span>
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
      {figure && typeof value === 'string' ? (
        <Size value={value} className="shrink-0 text-figure font-medium" />
      ) : (
        <span className="shrink-0 tabular-nums">{value}</span>
      )}
    </span>
  )
}

/** One named block of the fullness bar. */
export interface UsagePart {
  name: string
  bytes: number
}

/**
 * Fullness bar, 20 tall. `used` fills a faint hairline track, so the free
 * space stays visible as the unfilled rest; `parts` (largest items
 * first) split the filled part, 2px apart so each reads as its own block.
 * Hovering a block names it and its size in a tip under the bar.
 */
export function UsageBar({
  total,
  used,
  parts = [],
  label,
  restName,
  usedName,
  freeName,
  format,
}: {
  total: number
  used: number
  parts?: UsagePart[]
  label: string
  /** Names the filled part the `parts` leave over. */
  restName: string
  /** Names the filled part when there are no `parts`. */
  usedName: string
  /** Names the unfilled track. */
  freeName: string
  format: (bytes: number) => string
}) {
  const [tip, setTip] = useState<{ name: string; size: string; left: number; width: number; bar: number } | null>(null)
  const pct = (bytes: number) => (total > 0 ? Math.min(100, (bytes / total) * 100) : 0)
  const partsSum = parts.reduce((a, b) => a + b.bytes, 0)
  const rest = Math.max(0, parts.length ? used - partsSum : used)
  const free = Math.max(0, total - used)
  const show = (name: string, bytes: number) => (e: PointerEvent<HTMLSpanElement>) => {
    const el = e.currentTarget
    const bar = el.parentElement?.clientWidth ?? 0
    setTip({ name, size: format(bytes), left: el.offsetLeft, width: el.offsetWidth, bar })
  }
  const block = (key: string, name: string, bytes: number, width: number, fill: string) => (
    <span
      key={key}
      onPointerEnter={show(name, bytes)}
      className={`h-full shrink-0 transition-[opacity] duration-100 ease-out hover:opacity-80 ${fill}`}
      style={{ width: `${width}%` }}
    />
  )
  // Keep the tip inside the bar: anchored left near the start, right near the end.
  const place = (t: NonNullable<typeof tip>) => {
    const mid = t.left + t.width / 2
    if (mid < t.bar * 0.25) return { left: Math.max(0, t.left) }
    if (mid > t.bar * 0.75) return { right: Math.max(0, t.bar - t.left - t.width) }
    return { left: mid, transform: 'translateX(-50%)' }
  }
  return (
    <div className="relative" onPointerLeave={() => setTip(null)}>
      <div role="img" aria-label={label} className="h-5 w-full overflow-hidden rounded-tag bg-hairline">
        {/* Fills once from the left when it first shows (DESIGN_MAP → Motion). */}
        <div className="flex h-full w-full gap-0.5 enter-fill">
          {parts.map((part, i) => block(String(i), part.name, part.bytes, pct(part.bytes), 'bg-ink-2'))}
          {rest > 0 && block('rest', parts.length ? restName : usedName, rest, pct(rest), parts.length ? 'bg-ink-3' : 'bg-ink-2')}
          {free > 0 && block('free', freeName, free, pct(free), 'flex-1 bg-transparent')}
        </div>
      </div>
      {tip && (
        <span
          aria-hidden
          style={place(tip)}
          className="pointer-events-none absolute top-full z-40 mt-1.5 flex items-center gap-2 rounded-control bg-ink px-2 py-1 text-meta whitespace-nowrap text-page enter-fade"
        >
          <bdi>{tip.name}</bdi>
          <span className="tabular-nums opacity-70">{tip.size}</span>
        </span>
      )}
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

const SKELETON_WIDTHS = [46, 32, 58, 38, 50, 28]

/**
 * Still placeholder rows while a list loads, 32 tall like the real ones.
 * No pulse (DESIGN_MAP → Don'ts); the label is for screen readers.
 */
export function SkeletonRows({ label, rows = 6 }: { label: string; rows?: number }) {
  return (
    <div role="status" className="enter-fade">
      <span className="sr-only">{label}</span>
      {Array.from({ length: rows }, (_, i) => (
        <div key={i} aria-hidden className="flex h-8 items-center justify-between gap-4 px-3">
          <span className="h-2.5 rounded-full bg-raised" style={{ width: `${SKELETON_WIDTHS[i % SKELETON_WIDTHS.length]}%` }} />
          <span className="h-2.5 w-14 rounded-full bg-raised" />
        </div>
      ))}
    </div>
  )
}

/** An empty list's line, with the one step that helps next. */
export function EmptyLine({ text, action, onAction }: { text: string; action?: string; onAction?: () => void }) {
  return (
    <div role="status" className="flex flex-wrap items-center gap-x-3 gap-y-1 px-3 py-2 text-ink-2">
      <p>{text}</p>
      {action && onAction && (
        <button
          type="button"
          onClick={onAction}
          className="inline-flex min-h-6 cursor-pointer items-center rounded-tag px-1 text-ink underline-offset-2 hover:underline"
        >
          {action}
        </button>
      )}
    </div>
  )
}

/**
 * A small set of exclusive choices (views, kinds). One tab stop: Tab lands on
 * the chosen one, ← → (and Home, End) move and choose, as in a radio group.
 */
export function Segmented<K extends string>({
  label,
  options,
  value,
  onChange,
  className = '',
}: {
  label: string
  options: readonly { key: K; text: string }[]
  value: K
  onChange: (key: K) => void
  className?: string
}) {
  const refs = useRef<(HTMLButtonElement | null)[]>([])
  const at = options.findIndex((o) => o.key === value)
  const go = (i: number) => {
    const n = (i + options.length) % options.length
    onChange(options[n].key)
    refs.current[n]?.focus()
  }
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const next = { ArrowRight: at + 1, ArrowDown: at + 1, ArrowLeft: at - 1, ArrowUp: at - 1, Home: 0, End: options.length - 1 }[
      e.key
    ]
    if (next === undefined) return
    e.preventDefault()
    e.stopPropagation()
    go(next)
  }
  return (
    <div role="radiogroup" aria-label={label} onKeyDown={onKeyDown} className={`flex gap-1 self-start rounded-control bg-deepest p-0.5 ${className}`}>
      {options.map((o, i) => (
        <button
          key={o.key}
          ref={(el) => {
            refs.current[i] = el
          }}
          type="button"
          role="radio"
          aria-checked={o.key === value}
          tabIndex={o.key === value ? 0 : -1}
          onClick={() => onChange(o.key)}
          className={`h-7 cursor-pointer rounded-tag px-3 text-secondary transition-[background-color,color] duration-150 ease-out motion-reduce:transition-none ${
            o.key === value ? 'bg-card text-ink shadow-card' : 'text-ink-2 hover:text-ink'
          }`}
        >
          {o.text}
        </button>
      ))}
    </div>
  )
}

/**
 * A short label (and its keys) under a control, for buttons that show only an
 * icon. Shows after a short wait on hover, at once on keyboard focus. It
 * repeats the control's own accessible name, so it is hidden from screen
 * readers; the control carries `aria-label` and `aria-keyshortcuts`.
 */
type Align = 'start' | 'center' | 'end'

export function Tooltip({
  text,
  keys,
  align = 'center',
  side = 'bottom',
  children,
}: {
  text: string
  keys?: string
  /** `start` and `end` keep a tip by the window's edge inside it. */
  align?: Align
  /** `top` for controls at the bottom of the window. */
  side?: 'top' | 'bottom'
  children: ReactNode
}) {
  // Full class names, so Tailwind finds them.
  const place = {
    top: { start: 'left-0 origin-bottom-left', center: 'left-1/2 origin-bottom -translate-x-1/2', end: 'right-0 origin-bottom-right' },
    bottom: { start: 'left-0 origin-top-left', center: 'left-1/2 origin-top -translate-x-1/2', end: 'right-0 origin-top-right' },
  }[side][align]
  const y = side === 'top' ? 'bottom-full mb-1.5' : 'top-full mt-1.5'
  return (
    <span className="group/tip relative inline-flex">
      {children}
      <span
        aria-hidden
        className={`pointer-events-none absolute z-40 flex scale-97 items-center gap-2 rounded-control bg-ink px-2 py-1 text-meta whitespace-nowrap text-page opacity-0 transition-[opacity,scale] duration-100 ease-out group-hover/tip:scale-100 group-hover/tip:opacity-100 group-hover/tip:delay-400 group-has-focus-visible/tip:scale-100 group-has-focus-visible/tip:opacity-100 group-has-focus-visible/tip:delay-0 motion-reduce:scale-100 motion-reduce:transition-[opacity] ${y} ${place}`}
      >
        {text}
        {keys && <kbd className="font-mono opacity-70">{keys}</kbd>}
      </span>
    </span>
  )
}

type IconButtonProps = Omit<ComponentProps<'button'>, 'children'> & {
  /** The accessible name, also shown in the tooltip. */
  label: string
  /** Keys as shown to the eye (`⌘C`); `aria-keyshortcuts` carries them for screen readers. */
  keys?: string
  icon: ReactNode
  /** `quiet` drops the border, for the header. */
  look?: 'secondary' | 'quiet'
  align?: Align
  side?: 'top' | 'bottom'
}

/** A square 32 button with only an icon; the label lives in its tooltip. */
export function IconButton({
  label,
  keys,
  icon,
  look = 'secondary',
  align,
  side,
  className = '',
  ...props
}: IconButtonProps) {
  const button =
    look === 'quiet' ? (
      <button
        type="button"
        aria-label={label}
        className={`inline-flex size-8 shrink-0 cursor-pointer items-center justify-center rounded-control text-ink-2 transition-[background-color,color] duration-150 ease-out hover:bg-raised hover:text-ink motion-reduce:transition-none ${className}`}
        {...props}
      >
        {icon}
      </button>
    ) : (
      <Button aria-label={label} className={`w-8 justify-center px-0! ${className}`} {...props}>
        {icon}
      </Button>
    )
  return (
    <Tooltip text={label} keys={keys} align={align} side={side}>
      {button}
    </Tooltip>
  )
}
