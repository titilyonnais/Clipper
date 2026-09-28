import { useEffect, useState } from "react";
import { Keyboard } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/ui/button";

const MODIFIERS = new Set(["ControlLeft", "ControlRight", "ShiftLeft", "ShiftRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight"]);

/** Physical key code -> accelerator key name understood by the backend. */
function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^F\d{1,2}$/.test(code)) return code;
  const named = ["Space", "Enter", "Tab", "Backquote", "Insert", "Home", "End", "PageUp", "PageDown"];
  return named.includes(code) ? code : null;
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
    <div className="flex items-center gap-2">
      <button
        type="button"
        onClick={() => setRecording(true)}
        className={cn(
          "flex h-8 min-w-44 items-center gap-2 rounded-ctl border px-3 font-mono text-13 transition-colors",
          recording ? "border-foreground/40 bg-secondary text-foreground" : "border-input/70 bg-muted/60 text-foreground hover:border-input",
        )}
      >
        <Keyboard className="size-4 text-muted-foreground" />
        {recording ? <span className="text-muted-foreground">Appuyez sur la combinaison…</span> : value || <span className="text-subtle-foreground">Aucun</span>}
      </button>
      {value && !recording && (
        <Button size="sm" variant="ghost" onClick={() => onChange("")}>
          Désactiver
        </Button>
      )}
    </div>
  );
}
