import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { CalendarDays, Check, ChevronLeft, ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";
import { IconButton } from "@/ui/button";
import type { TimeRange } from "@/types";

const PRESETS: { value: TimeRange; label: string }[] = [
  { value: null, label: "Toutes les dates" },
  { value: "today", label: "Aujourd'hui" },
  { value: "week", label: "7 derniers jours" },
  { value: "month", label: "30 derniers jours" },
  { value: "3m", label: "3 derniers mois" },
  { value: "6m", label: "6 derniers mois" },
  { value: "1y", label: "12 derniers mois" },
];

const WEEKDAYS = ["lu", "ma", "me", "je", "ve", "sa", "di"];

/** Local calendar day as `YYYY-MM-DD` (the format the backend expects). */
function iso(d: Date) {
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
}

function parse(day: string) {
  const [y, m, d] = day.split("-").map(Number);
  return new Date(y, m - 1, d);
}

/** Bounds of a custom range (`day` or `from..to`), `null` for a preset. */
function customBounds(range: TimeRange): [string, string] | null {
  if (!range || !/^\d{4}-\d{2}-\d{2}/.test(range)) return null;
  const [a, b = a] = range.split("..");
  return a <= b ? [a, b] : [b, a];
}

function formatSpan(a: string, b: string) {
  const thisYear = new Date().getFullYear();
  const [from, to] = [parse(a), parse(b)];
  const fmt = new Intl.DateTimeFormat("fr-FR", {
    day: "numeric",
    month: "short",
    year: from.getFullYear() === thisYear && to.getFullYear() === thisYear ? undefined : "numeric",
  });
  return a === b ? fmt.format(from) : fmt.formatRange(from, to);
}

export function rangeLabel(range: TimeRange) {
  const custom = customBounds(range);
  if (custom) return formatSpan(...custom);
  return PRESETS.find((p) => p.value === range)?.label ?? "Période";
}

/** Calendar button of the history toolbar and its panel: common periods, or two days picked on a calendar. */
export function DateFilter({ value, onChange }: { value: TimeRange; onChange: (range: TimeRange) => void }) {
  const trigger = useRef<HTMLButtonElement>(null);
  const [open, setOpen] = useState(false);
  return (
    <>
      <IconButton
        ref={trigger}
        label={value ? `Période : ${rangeLabel(value)}` : "Filtrer par date"}
        size="lg"
        variant={value ? "secondary" : "outline"}
        active={!!value}
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
        className={cn("relative", !value && "border-input/70 bg-muted/60")}
      >
        <CalendarDays />
        {value && <span className="absolute top-1.5 right-1.5 size-1.5 rounded-full bg-foreground" />}
      </IconButton>
      {open && trigger.current && (
        <DatePanel
          trigger={trigger.current}
          value={value}
          onChange={(r) => {
            setOpen(false);
            onChange(r);
          }}
          onClose={() => setOpen(false)}
        />
      )}
    </>
  );
}

function DatePanel({
  trigger,
  value,
  onChange,
  onClose,
}: {
  trigger: HTMLElement;
  value: TimeRange;
  onChange: (range: TimeRange) => void;
  onClose: () => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  // `arrow`: horizontal position of the pointer to the button, in the panel.
  const [pos, setPos] = useState<{ left: number; top: number; arrow: number; below: boolean } | null>(null);
  const custom = customBounds(value);
  const today = iso(new Date());
  const [month, setMonth] = useState(() => {
    const d = custom ? parse(custom[1]) : new Date();
    return new Date(d.getFullYear(), d.getMonth(), 1);
  });
  // First day picked, waiting for the second.
  const [start, setStart] = useState<string | null>(null);
  const [hover, setHover] = useState<string | null>(null);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    // Centred under the row of the search field and the button, pointing at the button.
    const r = trigger.getBoundingClientRect();
    const row = (trigger.parentElement ?? trigger).getBoundingClientRect();
    const { width, height } = el.getBoundingClientRect();
    const margin = 8;
    const left = Math.max(margin, Math.min(row.left + row.width / 2 - width / 2, window.innerWidth - width - margin));
    const below = r.bottom + 10 + height + margin <= window.innerHeight;
    const top = below ? r.bottom + 10 : Math.max(margin, r.top - 10 - height);
    const arrow = Math.max(16, Math.min(r.left + r.width / 2 - left, width - 16));
    setPos({ left, top, arrow, below });
    el.focus();
  }, [trigger]);

  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      const t = e.target as Node;
      // The trigger toggles the panel itself.
      if (!ref.current?.contains(t) && !trigger.contains(t)) onClose();
    };
    document.addEventListener("mousedown", onDown, true);
    window.addEventListener("blur", onClose);
    return () => {
      document.removeEventListener("mousedown", onDown, true);
      window.removeEventListener("blur", onClose);
    };
  }, [trigger, onClose]);

  // Six weeks starting on the Monday before the 1st.
  const days = useMemo(() => {
    const first = new Date(month);
    first.setDate(1 - ((first.getDay() + 6) % 7));
    return Array.from({ length: 42 }, (_, i) => {
      const d = new Date(first);
      d.setDate(first.getDate() + i);
      return d;
    });
  }, [month]);

  // Highlighted span: the one being picked, else the active custom range.
  const span: [string, string] | null = start
    ? [start, hover ?? start].sort() as [string, string]
    : custom;

  const pick = (day: string) => {
    if (!start) {
      setStart(day);
      return;
    }
    const [a, b] = [start, day].sort();
    onChange(a === b ? a : `${a}..${b}`);
  };

  const shift = (n: number) => setMonth((m) => new Date(m.getFullYear(), m.getMonth() + n, 1));
  const nextDisabled = iso(new Date(month.getFullYear(), month.getMonth() + 1, 1)) > today;

  return createPortal(
    <div
      ref={ref}
      role="dialog"
      aria-label="Filtrer par date"
      tabIndex={-1}
      onKeyDown={(e) => {
        e.stopPropagation();
        if (e.key === "Escape") {
          e.preventDefault();
          if (start) setStart(null);
          else onClose();
        }
      }}
      style={{ left: pos?.left ?? 0, top: pos?.top ?? 0, visibility: pos ? "visible" : "hidden" }}
      className={cn(
        "fixed z-50 flex animate-pop rounded-dialog border border-border bg-popover shadow-lg shadow-black/30 outline-none",
        pos?.below === false ? "origin-bottom" : "origin-top",
      )}
    >
      {pos && (
        <span
          aria-hidden="true"
          style={{ left: pos.arrow - 6 }}
          className={cn(
            "absolute size-3 rotate-45 border-border bg-popover",
            pos.below ? "-top-[6.5px] border-t border-l" : "-bottom-[6.5px] border-r border-b",
          )}
        />
      )}
      <div className="flex w-44 flex-col gap-px border-r border-line p-1.5">
        {PRESETS.map((p) => {
          const checked = p.value === value;
          return (
            <button
              key={p.label}
              type="button"
              onClick={() => onChange(p.value)}
              className={cn(
                "flex h-8 items-center gap-2 rounded-[5px] px-2 text-left text-13 transition-colors duration-100",
                checked ? "bg-secondary text-foreground" : "text-muted-foreground hover:bg-muted hover:text-foreground",
              )}
            >
              <span className="flex-1 truncate">{p.label}</span>
              {checked && <Check className="size-3.5 shrink-0" />}
            </button>
          );
        })}
      </div>

      <div className="w-64 p-3" onMouseLeave={() => setHover(null)}>
        <div className="mb-2 flex items-center justify-between">
          <IconButton label="Mois précédent" size="sm" onClick={() => shift(-1)}>
            <ChevronLeft />
          </IconButton>
          <span className="text-13 font-medium text-foreground first-letter:uppercase">
            {month.toLocaleDateString("fr-FR", { month: "long", year: "numeric" })}
          </span>
          <IconButton label="Mois suivant" size="sm" onClick={() => shift(1)} disabled={nextDisabled}>
            <ChevronRight />
          </IconButton>
        </div>
        <div className="grid grid-cols-7 text-center text-[11px] text-subtle-foreground">
          {WEEKDAYS.map((d) => (
            <span key={d} className="flex h-7 items-center justify-center">
              {d}
            </span>
          ))}
        </div>
        <div className="grid grid-cols-7 gap-y-0.5">
          {days.map((d) => {
            const day = iso(d);
            const outside = d.getMonth() !== month.getMonth();
            const future = day > today;
            const inSpan = !!span && day >= span[0] && day <= span[1];
            const edge = !!span && (day === span[0] || day === span[1]);
            return (
              <div
                key={day}
                className={cn(
                  "flex h-8 items-center justify-center",
                  inSpan && "bg-secondary",
                  span && day === span[0] && "rounded-l-ctl",
                  span && day === span[1] && "rounded-r-ctl",
                )}
              >
                <button
                  type="button"
                  disabled={future}
                  onClick={() => pick(day)}
                  onMouseEnter={() => setHover(day)}
                  aria-label={d.toLocaleDateString("fr-FR", { dateStyle: "full" })}
                  aria-pressed={edge}
                  className={cn(
                    "size-8 rounded-ctl text-13 tabular-nums transition-colors duration-100 disabled:pointer-events-none disabled:opacity-30",
                    edge
                      ? "bg-foreground font-medium text-background"
                      : cn(
                          "hover:bg-muted hover:text-foreground",
                          outside ? "text-subtle-foreground" : "text-foreground",
                          day === today && "font-semibold underline decoration-foreground/40 underline-offset-4",
                        ),
                  )}
                >
                  {d.getDate()}
                </button>
              </div>
            );
          })}
        </div>
        <p className="mt-2.5 h-4 text-center text-xs text-subtle-foreground">
          {start ? "Choisissez le dernier jour" : custom ? formatSpan(...custom) : "Choisissez le premier jour"}
        </p>
      </div>
    </div>,
    document.body,
  );
}
