import { useEffect, useState } from "react";
import { api } from "./api";
import type { SettingsView } from "@/types";

/** Whether the configured AI provider is usable (Ollama running, key set…). */
export function useAiOnline(settings: SettingsView | null) {
  const [online, setOnline] = useState(false);
  const provider = settings?.ai_provider;
  const deps = [provider, settings?.ollama_url, settings?.ollama_model, settings?.openai_key_set, settings?.anthropic_key_set];
  useEffect(() => {
    if (!provider) return;
    let alive = true;
    const check = () => api.aiHealth().then((r) => alive && setOnline(r.ok)).catch(() => alive && setOnline(false));
    check();
    // Ollama may be started later: check again from time to time.
    const t = provider === "ollama" ? setInterval(check, 60_000) : undefined;
    return () => {
      alive = false;
      clearInterval(t);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  return online;
}
