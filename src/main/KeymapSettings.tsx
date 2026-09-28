import { useEffect, useState } from "react";
import { RotateCcw } from "lucide-react";
import { useSettings } from "@/lib/settings";
import { ACTIONS, RESERVED, comboKeys, comboOf, useKeymap, type ActionId } from "@/lib/shortcuts";
import { cn } from "@/lib/utils";
import { Button, IconButton } from "@/ui/button";
import { Kbd } from "@/ui/misc";
import { toast } from "@/ui/toast";

/** Keys that make sense alone; anything else needs Ctrl or Alt. */
const ALONE = /^(F\d{1,2}|Delete|Insert|Backspace)$/;

/** Editable list of the main-window shortcuts, grouped. */
export function KeymapSettings({ group }: { group: string }) {
  const { settings, update } = useSettings();
  const keymap = useKeymap();
  const [recording, setRecording] = useState<ActionId | null>(null);
  const custom = settings?.shortcuts ?? {};

  const save = async (id: ActionId, combo: string | null) => {
    const next = { ...custom };
    if (combo === null) delete next[id];
    else next[id] = combo;
    const error = await update({ shortcuts: next });
    if (error) toast(error, true);
  };

  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") return setRecording(null);
      const combo = comboOf(e);
      if (!combo) return;
      const key = combo.split("+").pop()!;
      if (RESERVED.has(combo)) return toast("Cette touche sert à naviguer, elle ne peut pas être attribuée.", true);
      if (!e.ctrlKey && !e.altKey && !ALONE.test(key)) {
        return toast("Ajoutez Ctrl ou Alt : seule, cette touche servirait à taper du texte.", true);
      }
      const taken = ACTIONS.find((a) => a.id !== recording && keymap[a.id] === combo);
      if (taken) return toast(`Déjà utilisé par « ${taken.label} ».`, true);
      const id = recording;
      setRecording(null);
      save(id, combo === ACTIONS.find((a) => a.id === id)!.default ? null : combo);
    };
    const cancel = () => setRecording(null);
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("blur", cancel);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("blur", cancel);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [recording, keymap]);

  return (
    <>
      {ACTIONS.filter((a) => a.group === group).map((a) => {
        const combo = keymap[a.id];
        const changed = custom[a.id] !== undefined;
        const active = recording === a.id;
        return (
          <div key={a.id} className="flex min-h-12 items-center justify-between gap-6 py-2">
            <span className="text-sm text-foreground">{a.label}</span>
            <div className="flex items-center gap-1">
              {changed && (
                <IconButton label={`Rétablir (${comboKeys(a.default).join("+")})`} size="sm" onClick={() => save(a.id, null)}>
                  <RotateCcw />
                </IconButton>
              )}
              <button
                type="button"
                onClick={() => setRecording(active ? null : a.id)}
                aria-label={`Raccourci : ${a.label}`}
                className={cn(
                  "-mr-2 flex h-8 min-w-36 items-center justify-end gap-1 rounded-ctl border px-2 transition-colors",
                  active ? "border-foreground/40 bg-secondary" : "border-transparent hover:border-input/70 hover:bg-muted/60",
                )}
              >
                {active ? (
                  <span className="px-1 text-13 text-muted-foreground">Appuyez sur les touches…</span>
                ) : combo ? (
                  comboKeys(combo).map((k) => <Kbd key={k}>{k}</Kbd>)
                ) : (
                  <span className="px-1 text-13 text-subtle-foreground">Aucun</span>
                )}
              </button>
            </div>
          </div>
        );
      })}
    </>
  );
}

export function ResetKeymap() {
  const { settings, update } = useSettings();
  const count = Object.keys(settings?.shortcuts ?? {}).length;
  if (!count) return null;
  return (
    <Button variant="ghost" size="sm" onClick={() => update({ shortcuts: {} })}>
      <RotateCcw /> Tout rétablir
    </Button>
  );
}
