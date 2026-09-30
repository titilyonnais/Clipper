import { useEffect, useRef } from "react";
import { createPortal } from "react-dom";
import { checkForUpdate, closeUpdate, installUpdate, noteBlocks, useUpdate } from "@/lib/update";
import { cn, humanBytes } from "@/lib/utils";
import { Logo } from "@/ui/misc";

/**
 * The update, in black whatever the theme: what is new, then a single
 * progress bar while it downloads and installs. Clipper then closes and
 * the installer starts it again.
 */
export function UpdateDialog() {
  const u = useUpdate();
  const ref = useRef<HTMLDivElement>(null);
  const working = u.status === "downloading" || u.status === "installing";

  useEffect(() => {
    if (!u.open) return;
    ref.current?.querySelector<HTMLElement>("[data-autofocus]")?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        closeUpdate();
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [u.open]);

  if (!u.open || !u.info) return null;
  const { info } = u;
  const blocks = info.notes ? noteBlocks(info.notes) : [];
  const percent = u.total ? Math.min(100, Math.round((u.downloaded / u.total) * 100)) : null;

  const retry = async () => {
    if (await checkForUpdate()) installUpdate();
  };

  return createPortal(
    <div
      className="fixed inset-0 z-40 flex animate-in items-center justify-center bg-black/80 p-6"
      onMouseDown={(e) => e.target === e.currentTarget && closeUpdate()}
    >
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={`Clipper ${info.version}`}
        tabIndex={-1}
        className="flex max-h-full w-full max-w-[420px] animate-dialog flex-col overflow-hidden rounded-[12px] border border-white/10 bg-black text-white shadow-2xl shadow-black/60 outline-none"
      >
        <div className="flex items-center gap-3.5 px-6 pt-6">
          <Logo className="size-10 shrink-0 text-white" />
          <div className="min-w-0">
            <div className="text-base">Clipper {info.version}</div>
            <div className="text-13 text-white/45">
              {working ? "Mise à jour en cours" : `Version installée : ${info.current}`}
            </div>
          </div>
        </div>

        {!working && blocks.length > 0 && (
          <div className="mx-6 mt-5 max-h-64 min-h-0 overflow-y-auto [scrollbar-width:thin] border-t border-white/10 pt-4 text-13 leading-relaxed text-white/70 [scrollbar-color:rgb(255_255_255/0.15)_transparent]">
            {blocks.map((b, i) =>
              b.kind === "h" ? (
                <h3 key={i} className={cn("mb-1.5 text-xs tracking-wide text-white/40 uppercase", i > 0 && "mt-4")}>
                  {b.text}
                </h3>
              ) : b.kind === "li" ? (
                <p key={i} className="relative mb-1.5 pl-3.5 before:absolute before:top-[0.6em] before:left-0 before:size-1 before:rounded-full before:bg-white/35">
                  {b.text}
                </p>
              ) : (
                <p key={i} className="mb-1.5">
                  {b.text}
                </p>
              ),
            )}
          </div>
        )}

        {working ? (
          <div className="px-6 pt-7 pb-7">
            <div className="h-[3px] overflow-hidden rounded-full bg-white/10">
              {u.status === "downloading" && percent !== null ? (
                <div className="h-full rounded-full bg-white transition-[width] duration-200 ease-out" style={{ width: `${percent}%` }} />
              ) : (
                <div className="h-full w-1/3 animate-indeterminate rounded-full bg-white" />
              )}
            </div>
            <div className="mt-3 flex justify-between text-xs text-white/45 tabular-nums">
              <span>{u.status === "installing" ? "Installation… Clipper va redémarrer" : "Téléchargement…"}</span>
              {u.status === "downloading" && (
                <span>
                  {percent !== null && `${percent} % · `}
                  {humanBytes(u.downloaded)}
                  {u.total ? ` / ${humanBytes(u.total)}` : ""}
                </span>
              )}
            </div>
          </div>
        ) : (
          <div className="px-6 pt-6 pb-6">
            {u.status === "error" && u.error && <p className="mb-4 text-13 text-red-400">{u.error}</p>}
            <div className="flex justify-end gap-2">
              <button
                type="button"
                onClick={closeUpdate}
                className="h-8 rounded-[7px] px-3.5 text-13 text-white/60 transition-colors hover:bg-white/10 hover:text-white"
              >
                Plus tard
              </button>
              <button
                type="button"
                data-autofocus
                onClick={u.status === "error" ? retry : installUpdate}
                className="h-8 rounded-[7px] bg-white px-4 text-13 text-black transition-colors hover:bg-white/85 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white/50"
              >
                {u.status === "error" ? "Réessayer" : "Mettre à jour"}
              </button>
            </div>
          </div>
        )}
      </div>
    </div>,
    document.body,
  );
}
