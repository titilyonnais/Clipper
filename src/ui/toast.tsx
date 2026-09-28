import { useEffect, useState } from "react";
import { AlertCircle, Check } from "lucide-react";
import { cn } from "@/lib/utils";

type Action = { label: string; run: () => void };
type Toast = { id: number; text: string; error: boolean; action?: Action };

let listeners: ((t: Toast) => void)[] = [];
let next = 1;

/** Short confirmation or error message at the bottom of the window, with an optional action. */
export function toast(text: string, error = false, action?: Action) {
  const t = { id: next++, text, error, action };
  listeners.forEach((l) => l(t));
}

export function ToastHost() {
  const [items, setItems] = useState<Toast[]>([]);
  useEffect(() => {
    const add = (t: Toast) => {
      setItems((list) => [...list.slice(-2), t]);
      setTimeout(() => setItems((list) => list.filter((x) => x.id !== t.id)), t.action ? 6000 : t.error ? 5000 : 1800);
    };
    listeners.push(add);
    return () => {
      listeners = listeners.filter((l) => l !== add);
    };
  }, []);
  return (
    <div className="pointer-events-none fixed inset-x-0 bottom-5 z-50 flex flex-col items-center gap-2" aria-live="polite">
      {items.map((t) => (
        <div
          key={t.id}
          className={cn(
            "pointer-events-auto flex max-w-md animate-toast items-center gap-2 rounded-ctl border border-border bg-popover py-2 pl-3 text-13 shadow-lg shadow-black/30",
            t.action ? "pr-1.5" : "pr-3",
            t.error ? "text-danger" : "text-foreground",
          )}
        >
          {t.error ? <AlertCircle className="size-4 shrink-0" /> : <Check className="size-4 shrink-0" />}
          <span>{t.text}</span>
          {t.action && (
            <button
              type="button"
              onClick={() => {
                setItems((list) => list.filter((x) => x.id !== t.id));
                t.action!.run();
              }}
              className="ml-2 h-7 rounded-[5px] px-2.5 font-medium text-foreground transition-colors hover:bg-secondary"
            >
              {t.action.label}
            </button>
          )}
        </div>
      ))}
    </div>
  );
}
