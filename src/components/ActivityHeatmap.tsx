import { useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api";
import { cn, localDay } from "@/lib/utils";

const WEEKS = 8;

interface Props {
  selected: string | null;
  onPick: (day: string | null) => void;
  refreshKey: number;
}

const LEVEL = ["bg-ink-800", "bg-accent/30", "bg-accent/55", "bg-accent/80", "bg-accent"];
const dayLabel = new Intl.DateTimeFormat("fr-FR", { weekday: "long", day: "numeric", month: "long" });

export function ActivityHeatmap({ selected, onPick, refreshKey }: Props) {
  const [counts, setCounts] = useState<Map<string, number>>(new Map());

  useEffect(() => {
    api.histogram(WEEKS * 7).then((rows) => setCounts(new Map(rows)));
  }, [refreshKey]);

  // Columns are weeks (Monday first), ending with the current week.
  const weeks = useMemo(() => {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const start = new Date(today);
    start.setDate(start.getDate() - ((today.getDay() + 6) % 7) - (WEEKS - 1) * 7);
    return Array.from({ length: WEEKS }, (_, w) =>
      Array.from({ length: 7 }, (_, d) => {
        const date = new Date(start);
        date.setDate(start.getDate() + w * 7 + d);
        return date;
      }),
    );
  }, [refreshKey]);

  const values = [...counts.values()];
  const max = Math.max(1, ...values);
  const total = values.reduce((a, b) => a + b, 0);
  const now = Date.now();

  if (total === 0) {
    return <p className="px-3 pb-2 text-[11px] text-ink-500">Pas encore d'activité.</p>;
  }

  return (
    <div className="px-3 pb-2">
      <div className="flex gap-[3px]">
        {weeks.map((week, wi) => (
          <div key={wi} className="flex flex-col gap-[3px]">
            {week.map((date) => {
              const day = localDay(date);
              const future = date.getTime() > now;
              const n = counts.get(day) ?? 0;
              const level = n === 0 ? 0 : Math.max(1, Math.ceil((n / max) * 4));
              return (
                <button
                  key={day}
                  disabled={future || n === 0}
                  onClick={() => onPick(selected === day ? null : day)}
                  title={future ? undefined : `${dayLabel.format(date)} : ${n} élément${n > 1 ? "s" : ""}`}
                  aria-label={future ? undefined : `${dayLabel.format(date)} : ${n}`}
                  className={cn(
                    "h-[13px] w-[13px] rounded-[3px]",
                    future ? "invisible" : LEVEL[level],
                    selected === day && "ring-1 ring-ink-50",
                  )}
                />
              );
            })}
          </div>
        ))}
      </div>
      <p className="mt-1.5 text-[10.5px] text-ink-500">
        {total} élément{total > 1 ? "s" : ""} sur {WEEKS} semaines
      </p>
    </div>
  );
}
