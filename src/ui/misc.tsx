import { useState } from "react";
import { cn, appIconUrl, appLabel } from "@/lib/utils";

export function Kbd({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <kbd
      className={cn(
        "inline-flex h-5 min-w-5 items-center justify-center rounded-ctl-sm bg-secondary px-1.5 font-sans text-[11px] font-medium text-muted-foreground",
        className,
      )}
    >
      {children}
    </kbd>
  );
}

type Tone = "neutral" | "ok" | "warn" | "danger";
const TONES: Record<Tone, string> = {
  neutral: "bg-secondary text-muted-foreground",
  ok: "bg-ok/10 text-ok",
  warn: "bg-warn/10 text-warn",
  danger: "bg-danger/10 text-danger",
};

export function Badge({
  children,
  tone = "neutral",
  className,
}: {
  children: React.ReactNode;
  tone?: Tone;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex h-5 shrink-0 items-center gap-1 rounded-ctl-sm px-1.5 text-xs font-medium [&_svg]:size-3",
        TONES[tone],
        className,
      )}
    >
      {children}
    </span>
  );
}

/** The Clipper mark: a rounded square holding three history lines. */
export function Logo({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 64 64" fill="none" aria-hidden="true" className={className}>
      <rect x="10" y="10" width="44" height="44" rx="11" stroke="currentColor" strokeWidth="5" />
      <path d="M21 25.5H43M21 32.5H38M21 39.5H31" stroke="currentColor" strokeWidth="4.5" strokeLinecap="round" />
    </svg>
  );
}

/** Icon of the application a clip came from, with a neutral fallback. */
export function AppIcon({
  dataDir,
  app,
  className,
}: {
  dataDir: string | undefined;
  app: string;
  className?: string;
}) {
  const [failed, setFailed] = useState(false);
  if (!dataDir || failed) {
    return (
      <span
        className={cn(
          "inline-flex items-center justify-center rounded-[4px] bg-secondary text-[9px] font-medium uppercase text-muted-foreground",
          className,
        )}
        aria-hidden="true"
      >
        {appLabel(app).charAt(0)}
      </span>
    );
  }
  return (
    <img
      src={appIconUrl(dataDir, app)}
      alt=""
      draggable={false}
      onError={() => setFailed(true)}
      className={cn("object-contain", className)}
    />
  );
}

export function EmptyState({
  icon,
  title,
  children,
}: {
  icon?: React.ReactNode;
  title: string;
  children?: React.ReactNode;
}) {
  return (
    <div className="flex h-full w-full flex-1 flex-col items-center justify-center px-8 text-center">
      {icon && <div className="mb-4 text-subtle-foreground [&_svg]:size-6">{icon}</div>}
      <p className="text-sm font-medium text-foreground">{title}</p>
      {children && <div className="mt-1.5 max-w-xs text-13 leading-relaxed text-muted-foreground">{children}</div>}
    </div>
  );
}

export function Spinner({ className }: { className?: string }) {
  return (
    <span
      role="status"
      aria-label="Chargement"
      className={cn(
        "inline-block size-3.5 animate-spin rounded-full border-2 border-muted-foreground/30 border-t-muted-foreground",
        className,
      )}
    />
  );
}
