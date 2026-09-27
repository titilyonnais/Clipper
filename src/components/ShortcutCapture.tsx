import { useEffect, useState } from "react";
import { Keyboard, X } from "lucide-react";
import { cn } from "@/lib/utils";

const MODIFIERS = new Set(["ControlLeft", "ControlRight", "ShiftLeft", "ShiftRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight"]);

/** Physical key code -> accelerator key name understood by the backend. */
function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  const named: Record<string, string> = {
    Space: "Space",
    Enter: "Enter",
    Tab: "Tab",
    Backquote: "Backquote",
    Insert: "Insert",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    ArrowUp: "ArrowUp",
    ArrowDown: "ArrowDown",
    ArrowLeft: "ArrowLeft",
    ArrowRight: "ArrowRight",
  };
  return named[code] ?? null;
}

export function ShortcutCapture({ value, onChange }: { value: string; onChange: (v: string) => void }) {
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.code === "Escape") return setRecording(false);
      if (MODIFIERS.has(e.code)) return;
      const key = keyName(e.code);
      const mods = [e.ctrlKey && "Ctrl", e.altKey && "Alt", e.shiftKey && "Shift", e.metaKey && "Super"].filter(Boolean);
      // A global shortcut without modifier would swallow a normal key everywhere.
      if (!key || (mods.length === 0 && !/^F\d/.test(key))) return;
      setRecording(false);
      onChange([...mods, key].join("+"));
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording, onChange]);

  return (
    <div className="flex flex-wrap items-center gap-2">
      <button
        onClick={() => setRecording(true)}
        className={cn(
          "flex h-9 min-w-[200px] items-center gap-2 rounded-md border px-3 font-mono text-[13px]",
          recording ? "animate-pulse border-accent bg-accent/15 text-accent" : "border-ink-700 bg-ink-800 text-ink-50",
        )}
      >
        <Keyboard size={13} />
        {recording ? "Appuyez sur la combinaison…" : value || <span className="italic text-ink-400">Désactivé</span>}
      </button>
      {recording ? (
        <span className="text-[11px] text-ink-400">Échap pour annuler</span>
      ) : (
        value && (
          <button
            onClick={() => onChange("")}
            className="inline-flex h-8 items-center gap-1 rounded-md border border-ink-700 bg-ink-800 px-2.5 text-[11.5px] text-ink-200 hover:text-red-400"
          >
            <X size={11} /> Désactiver
          </button>
        )
      )}
    </div>
  );
}
