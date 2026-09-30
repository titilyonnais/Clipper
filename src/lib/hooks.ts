import { useCallback, useEffect, useRef, useState } from "react";
import { listen, type EventCallback } from "@tauri-apps/api/event";
import { api } from "./api";
import type { ClipItem, ListParams } from "@/types";

/** Subscribe to a backend event for the component's lifetime. */
export function useTauriEvent<T>(name: string, handler: EventCallback<T>) {
  const ref = useRef(handler);
  ref.current = handler;
  useEffect(() => {
    const un = listen<T>(name, (e) => ref.current(e));
    return () => {
      un.then((f) => f());
    };
  }, [name]);
}

const PAGE = 80;

/**
 * Paginated clip list that refreshes itself when the history changes.
 * Responses are tagged so a slow, outdated query never overwrites a newer one.
 *
 * `stable`: on refresh, items already shown keep their place (a copied or
 * pinned item does not jump to the top under the pointer); only new items
 * are added at the top. `reorder()` applies the real order again.
 */
export function useClipList(params: ListParams | null, stable = false) {
  const [clips, setClips] = useState<ClipItem[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [loaded, setLoaded] = useState(false);
  // A reset outdates every request before it, a refresh the older resets
  // and refreshes; loading more outdates nothing, so a refresh is never
  // lost behind it (the pages it adds are deduplicated).
  const generation = useRef(0);
  const request = useRef(0);
  const clipsRef = useRef(clips);
  clipsRef.current = clips;
  const key = JSON.stringify(params);

  const load = useCallback(
    async (mode: "reset" | "refresh" | "more") => {
      if (!params) return;
      const gen = mode === "reset" ? ++generation.current : generation.current;
      const id = mode === "more" ? 0 : ++request.current;
      const offset = mode === "more" ? clipsRef.current.length : 0;
      const limit = mode === "refresh" ? Math.max(PAGE, clipsRef.current.length) : PAGE;
      const page = await api.list({ ...params, limit, offset });
      if (gen !== generation.current || (mode !== "more" && id !== request.current)) return;
      const prev = clipsRef.current;
      if (mode === "more") {
        const shown = new Set(prev.map((c) => c.id));
        setClips([...prev, ...page.filter((c) => !shown.has(c.id))]);
      } else if (mode === "refresh" && stable && prev.length) {
        const fresh = new Map(page.map((c) => [c.id, c]));
        const shown = new Set(prev.map((c) => c.id));
        const kept = prev.flatMap((c) => fresh.get(c.id) ?? []);
        setClips([...page.filter((c) => !shown.has(c.id)), ...kept]);
      } else {
        setClips(page);
      }
      setHasMore(page.length === limit);
      setLoaded(true);
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [key],
  );

  useEffect(() => {
    load("reset");
  }, [load]);

  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useTauriEvent("clips:changed", () => {
    // Bursts (several copies in a row) are coalesced.
    clearTimeout(timer.current);
    timer.current = setTimeout(() => load("refresh"), 50);
  });
  useEffect(() => () => clearTimeout(timer.current), []);

  return {
    clips,
    hasMore,
    loaded,
    loadMore: () => load("more"),
    refresh: () => load("refresh"),
    reorder: () => load("reset"),
  };
}

/** Value that follows `value` after `ms` without changes. */
export function useDebounced<T>(value: T, ms: number): T {
  const [v, setV] = useState(value);
  useEffect(() => {
    const t = setTimeout(() => setV(value), ms);
    return () => clearTimeout(t);
  }, [value, ms]);
  return v;
}
