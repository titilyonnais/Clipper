import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { IconButton } from "./button";
import { cn } from "@/lib/utils";

export function Dialog({
  title,
  description,
  onClose,
  children,
  footer,
  className,
}: {
  title: string;
  description?: React.ReactNode;
  onClose: () => void;
  children?: React.ReactNode;
  footer?: React.ReactNode;
  className?: string;
}) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = ref.current;
    (el?.querySelector<HTMLElement>("[autofocus], input, textarea, button") ?? el)?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose]);

  return createPortal(
    <div
      className="fixed inset-0 z-40 flex animate-in items-center justify-center bg-black/65 p-6 [.light_&]:bg-black/25"
      onMouseDown={(e) => e.target === e.currentTarget && onClose()}
    >
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
        className={cn(
          "flex max-h-full w-full max-w-md animate-dialog flex-col rounded-dialog border border-border bg-surface shadow-2xl shadow-black/40 outline-none",
          className,
        )}
      >
        <div className="flex items-start justify-between gap-4 px-5 pt-5">
          <div>
            <h2 className="text-base text-foreground">{title}</h2>
            {description && <p className="mt-1 text-13 leading-relaxed text-muted-foreground">{description}</p>}
          </div>
          <IconButton label="Fermer" size="sm" onClick={onClose} className="-mt-1 -mr-2">
            <X />
          </IconButton>
        </div>
        {children && <div className="min-h-0 overflow-auto px-5 pt-4">{children}</div>}
        <div className="flex justify-end gap-2 px-5 pt-5 pb-5">{footer}</div>
      </div>
    </div>,
    document.body,
  );
}
