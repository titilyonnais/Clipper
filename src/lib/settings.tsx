import { createContext, useCallback, useContext, useEffect, useRef, useState } from "react";
import { api, errorText } from "./api";
import { useTauriEvent } from "./hooks";
import type { Settings, SettingsView } from "@/types";

interface Ctx {
  settings: SettingsView | null;
  /** Save a change; resolves with an error message, or null on success. */
  update: (patch: Partial<Settings>) => Promise<string | null>;
  reload: () => void;
}

const SettingsContext = createContext<Ctx>({
  settings: null,
  update: async () => null,
  reload: () => {},
});

export function SettingsProvider({ children }: { children: React.ReactNode }) {
  const [settings, setState] = useState<SettingsView | null>(null);
  // Latest known settings and the chain of pending saves: two quick changes
  // are both kept, and saved in order.
  const latest = useRef<SettingsView | null>(null);
  const saving = useRef<Promise<unknown>>(Promise.resolve());
  const setSettings = useCallback((s: SettingsView | null) => {
    latest.current = s;
    setState(s);
  }, []);
  const reload = useCallback(() => {
    api.getSettings().then(setSettings);
  }, [setSettings]);

  useEffect(reload, [reload]);
  // Both windows share the settings: changes made in one reach the other.
  useTauriEvent("settings:changed", reload);
  useTauriEvent<string | null>("monitor:paused", (e) => {
    const s = latest.current;
    if (s) setSettings({ ...s, paused_until: e.payload, monitor_paused: e.payload === "forever" });
  });

  const update = useCallback(
    (patch: Partial<Settings>) => {
      const current = latest.current;
      if (!current) return Promise.resolve(null);
      const next = { ...current, ...patch };
      setSettings(next);
      const result = saving.current.then(async () => {
        try {
          const saved = await api.setSettings(latest.current ?? next);
          // A later change may already be on its way: keep the newest.
          if (latest.current === next) setSettings(saved);
          return null;
        } catch (e) {
          reload();
          return errorText(e);
        }
      });
      saving.current = result;
      return result;
    },
    [setSettings, reload],
  );

  useApplyAppearance(settings);

  return <SettingsContext.Provider value={{ settings, update, reload }}>{children}</SettingsContext.Provider>;
}

export const useSettings = () => useContext(SettingsContext);

function useApplyAppearance(settings: SettingsView | null) {
  const theme = settings?.theme ?? "auto";
  const density = settings?.density ?? "comfortable";
  useEffect(() => {
    const root = document.documentElement;
    const mq = window.matchMedia("(prefers-color-scheme: light)");
    const apply = () => {
      const light = theme === "light" || (theme === "auto" && mq.matches);
      root.classList.toggle("light", light);
      api.setFrameTheme(light).catch(() => {});
    };
    apply();
    root.classList.toggle("compact", density === "compact");
    if (theme !== "auto") return;
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [theme, density]);
}
