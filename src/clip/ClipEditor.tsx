import { useEffect, useRef, useState } from "react";
import { ClipboardPaste, Copy, Save } from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@/ui/button";
import { Textarea } from "@/ui/form";
import { Kbd } from "@/ui/misc";
import { run } from "./actions";
import type { ClipItem } from "@/types";

/** Edit a text clip, then paste it once or save it into the history. */
export function ClipEditor({
  clip,
  inPopup,
  onDone,
}: {
  clip: ClipItem;
  inPopup: boolean;
  onDone: (savedId?: number) => void;
}) {
  const [text, setText] = useState(clip.content ?? "");
  const ref = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    ref.current?.focus();
    ref.current?.setSelectionRange(0, 0);
  }, []);

  const paste = () => run(async () => {
    const outcome = await api.pasteText(text);
    onDone();
    return outcome;
  }, inPopup ? undefined : "Copié.");
  const save = () => run(async () => onDone(await api.updateText(clip.id, text)), "Enregistré.");

  return (
    <div
      className="flex h-full flex-col gap-3"
      onKeyDown={(e) => {
        if (e.key === "Escape") {
          e.preventDefault();
          e.stopPropagation();
          onDone();
        } else if (e.ctrlKey && e.key === "Enter") {
          e.preventDefault();
          e.stopPropagation();
          paste();
        } else if (e.ctrlKey && e.key.toLowerCase() === "s") {
          e.preventDefault();
          e.stopPropagation();
          save();
        }
      }}
    >
      <Textarea
        ref={ref}
        value={text}
        onChange={(e) => setText(e.target.value)}
        aria-label="Texte à modifier"
        className="min-h-0 flex-1 font-mono text-[12.5px]"
      />
      <div className="flex items-center gap-2">
        <Button variant="primary" size="sm" onClick={paste} disabled={!text.trim()}>
          {inPopup ? <ClipboardPaste /> : <Copy />} {inPopup ? "Coller" : "Copier"} <Kbd className="border-brand-foreground/30 text-brand-foreground/70">Ctrl ↵</Kbd>
        </Button>
        <Button size="sm" onClick={save} disabled={!text.trim() || text === clip.content}>
          <Save /> Enregistrer <Kbd>Ctrl S</Kbd>
        </Button>
        <Button size="sm" variant="ghost" onClick={() => onDone()} className="ml-auto">
          Annuler <Kbd>Échap</Kbd>
        </Button>
      </div>
    </div>
  );
}
