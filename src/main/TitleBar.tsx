import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Copy, EyeOff, ListOrdered, Minus, PanelLeftClose, PanelLeftOpen, Square, X } from "lucide-react";
import { api } from "@/lib/api";
import { useTauriEvent } from "@/lib/hooks";
import { useSettings } from "@/lib/settings";
import { comboLabel, useKeymap } from "@/lib/shortcuts";
import { cn } from "@/lib/utils";
import { Button, IconButton } from "@/ui/button";
import { Logo } from "@/ui/misc";
import type { QueueStatus } from "@/types";

/** Window title bar; the sidebar toggle sits at its left, above the sidebar icons. */
export function TitleBar({ sidebarFolded, onToggleSidebar }: { sidebarFolded: boolean; onToggleSidebar: () => void }) {
  const { settings } = useSettings();
  const keymap = useKeymap();
  const [maximized, setMaximized] = useState(false);
  const [queue, setQueue] = useState<QueueStatus | null>(null);
  // One object for the component's life: a new one each render would
  // subscribe again every time.
  const [w] = useState(getCurrentWindow);

  useEffect(() => {
    const sync = () => w.isMaximized().then(setMaximized).catch(() => {});
    sync();
    const un = w.onResized(sync);
    return () => {
      un.then((f) => f());
    };
  }, [w]);
  // The current state once, then its changes (a late answer never
  // overwrites a newer event).
  const queueEvents = useRef(0);
  useEffect(() => {
    const at = queueEvents.current;
    api.queueStatus().then((q) => queueEvents.current === at && setQueue(q));
  }, []);
  useTauriEvent<QueueStatus>("queue:changed", (e) => {
    queueEvents.current++;
    setQueue(e.payload);
  });

  const paused = settings?.paused_until;
  const pausedLabel =
    paused === "forever"
      ? "Capture suspendue"
      : paused
        ? `Incognito jusqu'à ${new Date(paused).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" })}`
        : null;

  return (
    <header className="flex h-11 shrink-0 items-center border-b border-line bg-background" data-tauri-drag-region>
      <div className="flex h-full items-center gap-2 pl-2.5" data-tauri-drag-region>
        <IconButton
          label={`${sidebarFolded ? "Déplier" : "Replier"} la barre latérale (${comboLabel(keymap.toggle_sidebar)})`}
          onClick={onToggleSidebar}
        >
          {sidebarFolded ? <PanelLeftOpen /> : <PanelLeftClose />}
        </IconButton>
        <Logo className="size-[18px] text-foreground" />
        <span className="text-13 font-medium tracking-tight" data-tauri-drag-region>
          Clipper
        </span>
      </div>
      <div className="flex h-full flex-1 items-center justify-center gap-2" data-tauri-drag-region>
        {queue?.active && (
          <div className="flex animate-in items-center gap-2 rounded-ctl bg-secondary py-0.5 pr-0.5 pl-2.5 text-xs">
            <ListOrdered className="size-3.5 text-muted-foreground" />
            <span>
              Collage en série · {queue.position + 1} sur {queue.total}
            </span>
            <Button size="xs" variant="ghost" onClick={() => api.stopQueue()}>
              Arrêter
            </Button>
          </div>
        )}
        {pausedLabel && (
          <div className="flex animate-in items-center gap-2 rounded-ctl bg-secondary py-0.5 pr-0.5 pl-2.5 text-xs">
            <EyeOff className="size-3.5 text-muted-foreground" />
            <span>{pausedLabel}</span>
            <Button size="xs" variant="ghost" onClick={() => api.setIncognito(null)}>
              Reprendre
            </Button>
          </div>
        )}
      </div>
      <div className="flex h-full">
        <WindowButton label="Réduire" onClick={() => w.minimize()}>
          <Minus />
        </WindowButton>
        <WindowButton label={maximized ? "Restaurer" : "Agrandir"} onClick={() => w.toggleMaximize()}>
          {maximized ? <Copy className="!size-3.5" /> : <Square className="!size-3.5" />}
        </WindowButton>
        <WindowButton label="Fermer (Clipper reste dans la zone de notification)" onClick={() => api.hideMain()} danger>
          <X />
        </WindowButton>
      </div>
    </header>
  );
}

function WindowButton({
  label,
  onClick,
  danger,
  children,
}: {
  label: string;
  onClick: () => void;
  danger?: boolean;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      onClick={onClick}
      className={cn(
        "flex h-full w-12 items-center justify-center text-muted-foreground transition-colors [&_svg]:size-4",
        danger ? "hover:bg-[#c42b1c] hover:text-white" : "hover:bg-muted hover:text-foreground",
      )}
    >
      {children}
    </button>
  );
}
