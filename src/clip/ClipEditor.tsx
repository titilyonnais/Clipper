import { useEffect, useRef, useState } from "react";
import { ClipboardPaste, Save } from "lucide-react";
import { api } from "@/lib/api";
import { Button } from "@/ui/button";
import { Textarea } from "@/ui/form";
import { Kbd } from "@/ui/misc";
import { run } from "./actions";
import type { ClipItem } from "@/types";

/**
 * Text area of the editor. Ctrl+S saves, Ctrl+Entrée uses the edited text
 * (copy or paste), Échap asks to leave.
 */
export function EditorArea({
  value,
  onChange,
  onSave,
  onUse,
  onCancel,
}: {
  value: string;
  onChange: (v: string) => void;
  onSave: () => void;
  onUse: () => void;
  onCancel: () => void;
}) {
  const ref = useRef<HTMLTextAreaElement>(null);
  useEffect(() => {
    ref.current?.focus();
    ref.current?.setSelectionRange(0, 0);
  }, []);
  return (
    <Textarea
      ref={ref}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      onKeyDown={(e) => {
        const key = e.key.toLowerCase();
        if (e.key === "Escape" || (e.ctrlKey && (key === "s" || e.key === "Enter"))) {
          e.preventDefault();
          e.stopPropagation();
          if (e.key === "Escape") onCancel();
          else if (key === "s") onSave();
          else onUse();
        }
      }}
      aria-label="Texte à modifier"
      className="min-h-0 flex-1 font-mono text-[12.5px]"
    />
  );
}

/** Editor of the quick-paste popup, with its actions below the text. */
export function ClipEditor({ clip, onDone }: { clip: ClipItem; onDone: () => void }) {
  const [text, setText] = useState(clip.content ?? "");
  const changed = text !== (clip.content ?? "");
  const paste = () => text.trim() && run(async () => {
    await api.pasteText(text);
    onDone();
  });
  const save = () => changed && text.trim() && run(async () => {
    await api.updateText(clip.id, text);
    onDone();
  }, "Enregistré.");

  return (
    <div className="flex h-full flex-col gap-3">
      <EditorArea value={text} onChange={setText} onSave={save} onUse={paste} onCancel={onDone} />
      <div className="flex items-center gap-2">
        <Button variant="primary" size="sm" onClick={paste} disabled={!text.trim()}>
          <ClipboardPaste /> Coller <Kbd className="bg-brand-foreground/15 text-brand-foreground/70">Ctrl Entrée</Kbd>
        </Button>
        <Button size="sm" onClick={save} disabled={!text.trim() || !changed}>
          <Save /> Enregistrer <Kbd>Ctrl S</Kbd>
        </Button>
        <Button size="sm" variant="ghost" onClick={onDone} className="ml-auto">
          Fermer <Kbd>Échap</Kbd>
        </Button>
      </div>
    </div>
  );
}
