import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Pause, Play, Search, Settings, Square, Copy, X } from "lucide-react";
import { cn } from "@/lib/utils";

interface Props {
  paused: boolean;
  onTogglePause: () => void;
  onOpenSettings: () => void;
  onSearch: () => void;
  count: number;
}

export function TitleBar({ paused, onTogglePause, onOpenSettings, onSearch, count }: Props) {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    const w = getCurrentWindow();
    const sync = () => w.isMaximized().then(setMaximized).catch(() => {});
    sync();
    const un = w.onResized(sync);
    return () => {
      un.then((f) => f());
    };
  }, []);

  const w = getCurrentWindow();

  return (
    <header className="flex h-9 shrink-0 select-none items-center border-b border-ink-700/60 bg-ink-950" data-tauri-drag-region>
      <div className="flex h-full items-center gap-2 px-3" data-tauri-drag-region>
        <img src="/clipper.svg" alt="" className="h-4 w-4" data-tauri-drag-region />
        <span className="font-display text-[12.5px] font-semibold text-ink-100" data-tauri-drag-region>
          Clipper
        </span>
        <span className="text-[11px] text-ink-500" data-tauri-drag-region>
          {count.toLocaleString("fr-FR")} élément{count > 1 ? "s" : ""}
        </span>
      </div>
      <div className="h-full flex-1" data-tauri-drag-region />

      <div className="flex items-center gap-0.5 px-2">
        <ToolButton onClick={onSearch} title="Rechercher (Ctrl+F)">
          <Search size={13} />
        </ToolButton>
        <ToolButton
          onClick={onTogglePause}
          title={paused ? "Reprendre la capture" : "Suspendre la capture"}
          active={paused}
        >
          {paused ? <Play size={13} /> : <Pause size={13} />}
        </ToolButton>
        <ToolButton onClick={onOpenSettings} title="Paramètres">
          <Settings size={13} />
        </ToolButton>
      </div>

      <div className="flex h-full items-center">
        <WindowButton onClick={() => w.minimize()} title="Réduire">
          <Minus size={12} />
        </WindowButton>
        <WindowButton onClick={() => w.toggleMaximize()} title={maximized ? "Restaurer" : "Agrandir"}>
          {maximized ? <Copy size={10} /> : <Square size={10} />}
        </WindowButton>
        <WindowButton onClick={() => w.hide()} title="Fermer (Clipper reste dans la zone de notification)" danger>
          <X size={13} />
        </WindowButton>
      </div>
    </header>
  );
}

function ToolButton({
  children,
  onClick,
  title,
  active,
}: {
  children: React.ReactNode;
  onClick: () => void;
  title: string;
  active?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      aria-label={title}
      className={cn(
        "flex h-7 w-7 items-center justify-center rounded-md hover:bg-ink-800",
        active ? "bg-accent/15 text-accent" : "text-ink-300 hover:text-ink-50",
      )}
    >
      {children}
    </button>
  );
}

function WindowButton({
  children,
  onClick,
  title,
  danger,
}: {
  children: React.ReactNode;
  onClick: () => void;
  title: string;
  danger?: boolean;
}) {
  return (
    <button
      onClick={onClick}
      title={title}
      aria-label={title}
      className={cn(
        "flex h-full w-11 items-center justify-center text-ink-300",
        danger ? "hover:bg-red-600 hover:text-white" : "hover:bg-ink-800",
      )}
    >
      {children}
    </button>
  );
}
