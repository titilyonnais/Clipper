import { useEffect, useRef, useState } from "react";
import { ClipboardPaste, Copy, Sparkles, X } from "lucide-react";
import { api, errorText } from "@/lib/api";
import { Button, IconButton } from "@/ui/button";
import type { MenuEntry } from "@/ui/menu";
import { run } from "./actions";
import type { AiAction, AiResponse, ClipItem } from "@/types";

const LANGUAGES = ["anglais", "français", "espagnol", "allemand", "italien", "portugais", "néerlandais", "japonais", "chinois"];

export function useAi(clip: ClipItem | null) {
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<AiResponse | null>(null);
  const request = useRef(0);
  useEffect(() => {
    request.current++;
    setBusy(false);
    setResult(null);
  }, [clip?.id]);

  const ask = async (action: AiAction | "tag", lang?: string) => {
    if (!clip) return;
    const id = ++request.current;
    setBusy(true);
    setResult(null);
    try {
      const r = action === "tag" ? await api.aiSmartTag(clip.id) : await api.aiRun(clip.id, action, lang);
      if (id === request.current) setResult(r);
    } catch (e) {
      if (id === request.current) setResult({ ok: false, text: "", error: errorText(e) });
    } finally {
      if (id === request.current) setBusy(false);
    }
  };
  return { busy, result, ask, dismiss: () => setResult(null) };
}

/** Actions offered by the AI menu. */
export function aiEntries(ai: ReturnType<typeof useAi>): MenuEntry[] {
  return [
    { heading: "Intelligence artificielle" },
    { label: "Résumer", onSelect: () => ai.ask("summarize") },
    { label: "Expliquer", onSelect: () => ai.ask("explain") },
    { label: "Reformuler", onSelect: () => ai.ask("rephrase") },
    { label: "Corriger l'orthographe", onSelect: () => ai.ask("fix") },
    { label: "Classer (collection et tags)", onSelect: () => ai.ask("tag") },
    { heading: "Traduire en" },
    ...LANGUAGES.map((l) => ({ label: l.charAt(0).toUpperCase() + l.slice(1), onSelect: () => ai.ask("translate", l) })),
  ];
}

/** Answer of the model, with paste/copy actions. */
export function AiResult({ ai, inPopup }: { ai: ReturnType<typeof useAi>; inPopup: boolean }) {
  const r = ai.result;
  if (!r && !ai.busy) return null;
  return (
    <div className="animate-pop rounded-card border border-border bg-surface p-3">
      <div className="mb-2 flex items-center gap-2 text-xs text-subtle-foreground">
        <Sparkles className="size-3.5" /> {ai.busy ? "Réponse en cours…" : "Réponse de l'IA"}
        {r && (
          <IconButton label="Fermer" size="xs" className="ml-auto" onClick={ai.dismiss}>
            <X />
          </IconButton>
        )}
      </div>
      {r?.ok && (
        <>
          <p className="selectable max-h-60 overflow-auto text-13 leading-relaxed whitespace-pre-wrap text-foreground">{r.text}</p>
          <div className="mt-3 flex gap-2">
            <Button size="xs" variant="primary" onClick={() => run(() => api.pasteText(r.text), inPopup ? undefined : "Copié.")}>
              {inPopup ? <ClipboardPaste /> : <Copy />} {inPopup ? "Coller" : "Copier"}
            </Button>
          </div>
        </>
      )}
      {r && !r.ok && <p className="text-13 text-danger">{r.error}</p>}
    </div>
  );
}
