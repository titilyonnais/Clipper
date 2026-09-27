import { useEffect } from "react";
import { X } from "lucide-react";
import { cn } from "@/lib/utils";

export function Dialog({
  title,
  onClose,
  width = "w-[600px]",
  children,
  footer,
}: {
  title: React.ReactNode;
  onClose: () => void;
  width?: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
}) {
  return (
    <div
      className="fixed inset-0 z-50 flex animate-fade-in items-center justify-center bg-black/55"
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
        role="dialog"
        aria-modal="true"
        className={cn(
          "flex max-h-[85vh] animate-scale-in flex-col overflow-hidden rounded-2xl border border-ink-700 bg-ink-900 shadow-2xl",
          width,
        )}
      >
        <div className="flex items-center justify-between border-b border-ink-700/60 px-6 py-3.5">
          <h2 className="font-display text-[16px] font-semibold text-ink-50">{title}</h2>
          <button
            onClick={onClose}
            aria-label="Fermer"
            className="flex h-8 w-8 items-center justify-center rounded-lg text-ink-300 hover:bg-ink-800"
          >
            <X size={15} />
          </button>
        </div>
        <div className="min-h-0 flex-1 overflow-auto">{children}</div>
        {footer && <div className="flex items-center gap-2 border-t border-ink-700/60 px-6 py-3">{footer}</div>}
      </div>
    </div>
  );
}

export function Field({ label, hint, children }: { label: string; hint?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div>
      <div className="mb-1.5 text-[12px] font-medium text-ink-200">{label}</div>
      {children}
      {hint && <div className="mt-1.5 text-[11px] leading-relaxed text-ink-500">{hint}</div>}
    </div>
  );
}

export function Chip({
  active,
  onClick,
  children,
  className,
}: {
  active: boolean;
  onClick: () => void;
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <button
      onClick={onClick}
      aria-pressed={active}
      className={cn(
        "h-8 rounded-md border px-3 text-[12px] font-medium",
        active
          ? "border-accent bg-accent text-[rgb(var(--on-accent))]"
          : "border-ink-700 bg-ink-800 text-ink-200 hover:bg-ink-700",
        className,
      )}
    >
      {children}
    </button>
  );
}

export function Toggle({ checked, onChange, label }: { checked: boolean; onChange: (v: boolean) => void; label: string }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={cn("relative h-6 w-10 shrink-0 rounded-full", checked ? "bg-accent" : "bg-ink-700")}
    >
      <span
        className={cn(
          "absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform",
          checked && "translate-x-4",
        )}
      />
    </button>
  );
}

export const inputClass =
  "h-9 w-full rounded-md border border-ink-700 bg-ink-800 px-3 text-[13px] text-ink-50 placeholder:text-ink-500";

export const buttonClass =
  "inline-flex h-9 items-center gap-1.5 rounded-md border border-ink-700 bg-ink-800 px-3 text-[12.5px] font-medium text-ink-200 hover:bg-ink-700 disabled:opacity-40";

/** Remove a transient message after a delay. */
export function useAutoClear<T>(value: T | null, set: (v: T | null) => void, ms = 4000) {
  useEffect(() => {
    if (value === null) return;
    const t = setTimeout(() => set(null), ms);
    return () => clearTimeout(t);
  }, [value, set, ms]);
}
