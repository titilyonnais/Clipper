import { forwardRef } from "react";
import { cn } from "@/lib/utils";

const FIELD =
  "w-full rounded-ctl border border-input/70 bg-muted/60 px-3 text-sm text-foreground placeholder:text-subtle-foreground " +
  "transition-[border-color,box-shadow] duration-150 hover:border-input " +
  "focus:border-foreground/40 focus:ring-3 focus:ring-foreground/10 disabled:opacity-50";

export const Input = forwardRef<HTMLInputElement, React.InputHTMLAttributes<HTMLInputElement>>(function Input(
  { className, ...props },
  ref,
) {
  return <input ref={ref} spellCheck={false} className={cn(FIELD, "h-8", className)} {...props} />;
});

export const Textarea = forwardRef<HTMLTextAreaElement, React.TextareaHTMLAttributes<HTMLTextAreaElement>>(
  function Textarea({ className, ...props }, ref) {
    return <textarea ref={ref} spellCheck={false} className={cn(FIELD, "resize-none py-2 leading-relaxed", className)} {...props} />;
  },
);

export function Switch({
  checked,
  onChange,
  label,
  disabled,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "relative h-5 w-9 shrink-0 rounded-full transition-colors duration-150 disabled:opacity-40",
        checked ? "bg-brand" : "bg-secondary",
      )}
    >
      <span
        className={cn(
          "absolute top-0.5 left-0.5 size-4 rounded-full transition-transform duration-150 ease-out-soft",
          checked ? "translate-x-4 bg-brand-foreground" : "bg-muted-foreground",
        )}
      />
    </button>
  );
}

/** Mutually exclusive options shown as a compact segmented control. */
export function Segmented<T extends string | number>({
  value,
  options,
  onChange,
  size = "md",
  label,
}: {
  value: T;
  options: { value: T; label: React.ReactNode; title?: string }[];
  onChange: (v: T) => void;
  size?: "sm" | "md";
  label: string;
}) {
  return (
    <div role="radiogroup" aria-label={label} className="inline-flex gap-0.5 rounded-ctl bg-muted p-0.5">
      {options.map((o) => (
        <button
          key={String(o.value)}
          type="button"
          role="radio"
          aria-checked={o.value === value}
          title={o.title}
          onClick={() => onChange(o.value)}
          className={cn(
            "inline-flex items-center gap-1.5 rounded-[5px] font-medium transition-colors duration-150 [&_svg]:size-3.5",
            size === "sm" ? "h-6 px-2 text-xs" : "h-7 px-3 text-13",
            o.value === value
              ? "bg-secondary text-foreground shadow-sm"
              : "text-muted-foreground hover:text-foreground",
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

/** Settings row: label and explanation on the left, control on the right. */
export function Row({
  label,
  hint,
  children,
  htmlFor,
}: {
  label: string;
  hint?: React.ReactNode;
  children: React.ReactNode;
  htmlFor?: string;
}) {
  return (
    <div className="flex items-start justify-between gap-8 py-3.5">
      <div className="min-w-0">
        <label htmlFor={htmlFor} className="block text-sm text-foreground">
          {label}
        </label>
        {hint && <p className="mt-0.5 text-13 leading-relaxed text-muted-foreground">{hint}</p>}
      </div>
      <div className="flex shrink-0 items-center gap-2 pt-0.5">{children}</div>
    </div>
  );
}

/** Label above a full-width control. */
export function Field({ label, hint, children }: { label: string; hint?: React.ReactNode; children: React.ReactNode }) {
  return (
    <div className="py-3.5">
      <div className="mb-2 text-sm text-foreground">{label}</div>
      {children}
      {hint && <p className="mt-2 text-13 leading-relaxed text-muted-foreground">{hint}</p>}
    </div>
  );
}
