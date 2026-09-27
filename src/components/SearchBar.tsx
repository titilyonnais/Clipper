import { forwardRef } from "react";
import { Search, X } from "lucide-react";

interface Props {
  value: string;
  onChange: (v: string) => void;
  count: number;
  more: boolean;
}

export const SearchBar = forwardRef<HTMLInputElement, Props>(function SearchBar({ value, onChange, count, more }, ref) {
  return (
    <div className="flex h-12 shrink-0 items-center gap-2.5 border-b border-ink-700/60 px-4">
      <Search size={16} className="shrink-0 text-ink-400" />
      <input
        ref={ref}
        type="search"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="Rechercher…"
        spellCheck={false}
        autoFocus
        className="min-w-0 flex-1 bg-transparent text-[14px] focus-visible:outline-none text-ink-50 placeholder:text-ink-500 [&::-webkit-search-cancel-button]:hidden"
      />
      {value && (
        <button onClick={() => onChange("")} className="rounded p-0.5 text-ink-400 hover:text-ink-50" aria-label="Effacer">
          <X size={13} />
        </button>
      )}
      <span className="font-mono text-[11px] tabular-nums text-ink-500">
        {count.toLocaleString("fr-FR")}
        {more ? "+" : ""}
      </span>
    </div>
  );
});
