import { createContext, useCallback, useContext, useEffect, useState } from "react";
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
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const reload = useCallback(() => {
    api.getSettings().then(setSettings);
  }, []);

  useEffect(reload, [reload]);
  // Both windows share the settings: changes made in one reach the other.
  useTauriEvent("settings:changed", reload);
  useTauriEvent<string | null>("monitor:paused", (e) =>
    setSettings((s) => (s ? { ...s, paused_until: e.payload, monitor_paused: e.payload === "forever" } : s)),
  );

  const update = useCallback(
    async (patch: Partial<Settings>) => {
      if (!settings) return null;
      const next = { ...settings, ...patch };
      setSettings(next);
      try {
        setSettings(await api.setSettings(next));
        return null;
      } catch (e) {
        setSettings(settings);
        return errorText(e);
      }
    },
    [settings],
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
