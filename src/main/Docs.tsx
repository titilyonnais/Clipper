import { Fragment, useEffect, useState } from "react";
import { ChevronRight } from "lucide-react";
import { cn } from "@/lib/utils";

type Block = { kind: "h" | "li" | "p"; text: string };
type Section = { title: string; blocks: Block[] };

/** "## Title" sections of a small Markdown text, with "###", "- " and paragraphs. */
function parse(markdown: string): Section[] {
  const sections: Section[] = [];
  for (const raw of markdown.split(/\r?\n/)) {
    const line = raw.trim();
    if (line.startsWith("## ")) sections.push({ title: line.slice(3), blocks: [] });
    else if (!line || line.startsWith("# ") || !sections.length) continue;
    else if (line.startsWith("### ")) sections.at(-1)!.blocks.push({ kind: "h", text: line.slice(4) });
    else if (/^[-*] /.test(line)) sections.at(-1)!.blocks.push({ kind: "li", text: line.slice(2) });
    else sections.at(-1)!.blocks.push({ kind: "p", text: line });
  }
  return sections;
}

/** **bold** and `code`; everything else is plain text. */
function Inline({ text }: { text: string }) {
  return (
    <>
      {text.split(/(\*\*[^*]+\*\*|`[^`]+`)/g).map((part, i) =>
        part.startsWith("**") ? (
          <span key={i} className="text-foreground">
            {part.slice(2, -2)}
          </span>
        ) : part.startsWith("`") ? (
          <code key={i} className="rounded-[4px] bg-secondary px-1 py-px font-mono text-xs text-foreground">
            {part.slice(1, -1)}
          </code>
        ) : (
          <Fragment key={i}>{part}</Fragment>
        ),
      )}
    </>
  );
}

/**
 * A Markdown text loaded on demand, as folding sections: nothing is loaded
 * nor laid out before the tab is opened.
 */
export function Docs({
  load,
  open = [],
  label,
  trailing,
}: {
  load: () => Promise<{ default: string }>;
  /** Titles of the sections shown unfolded. */
  open?: string[];
  label: string;
  /** Text shown after a section title, e.g. "installée". */
  trailing?: (title: string) => string | null;
}) {
  const [sections, setSections] = useState<Section[] | null>(null);
  useEffect(() => {
    let live = true;
    load().then((m) => live && setSections(parse(m.default)));
    return () => {
      live = false;
    };
  }, [load]);

  if (!sections) return <div className="h-40" aria-busy="true" />;
  return (
    <div className="divide-y divide-line rounded-card border border-line bg-surface" aria-label={label}>
      {sections.map((s) => (
        <details key={s.title} open={open.includes(s.title)} className="group">
          <summary className="flex h-11 cursor-pointer list-none items-center gap-2.5 px-4 text-sm text-foreground select-none [&::-webkit-details-marker]:hidden">
            <ChevronRight className="size-4 shrink-0 text-subtle-foreground transition-transform duration-150 group-open:rotate-90" />
            <span className="flex-1">{s.title}</span>
            {trailing?.(s.title) && <span className="text-xs text-subtle-foreground">{trailing(s.title)}</span>}
          </summary>
          <div className="px-4 pb-4 pl-[42px] text-13 leading-relaxed text-muted-foreground">
            {s.blocks.map((b, i) =>
              b.kind === "h" ? (
                <h3 key={i} className={cn("mb-1 text-xs text-subtle-foreground", i > 0 && "mt-3")}>
                  {b.text}
                </h3>
              ) : b.kind === "li" ? (
                <p key={i} className="relative mb-1 pl-3.5 before:absolute before:top-[0.6em] before:left-0 before:size-1 before:rounded-full before:bg-subtle-foreground">
                  <Inline text={b.text} />
                </p>
              ) : (
                <p key={i} className="mb-1">
                  <Inline text={b.text} />
                </p>
              ),
            )}
          </div>
        </details>
      ))}
    </div>
  );
}

export const loadGuide = () => import("../../docs/GUIDE.md?raw");
export const loadChangelog = () => import("../../CHANGELOG.md?raw");
