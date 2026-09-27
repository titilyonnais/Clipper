import { useState } from "react";
import { Check, Cpu, Database, Download, Info, Keyboard, Palette, ShieldCheck, Trash2, Upload, type LucideIcon } from "lucide-react";
import { api, errorText } from "@/lib/api";
import { cn, humanBytes } from "@/lib/utils";
import type { AiProvider, Settings as SettingsT, SettingsView, Stats } from "@/types";
import { ShortcutCapture } from "./ShortcutCapture";
import { Chip, Dialog, Field, Toggle, buttonClass, inputClass, useAutoClear } from "./ui";


type Tab = "general" | "capture" | "retention" | "ai" | "appearance" | "data" | "about";

const TABS: [Tab, string, LucideIcon][] = [
  ["general", "Général", Keyboard],
  ["capture", "Capture", ShieldCheck],
  ["retention", "Rétention", Database],
  ["ai", "IA", Cpu],
  ["appearance", "Apparence", Palette],
  ["data", "Données", Download],
  ["about", "À propos", Info],
];

const RETENTION: [number, string][] = [
  [0, "Jamais"],
  [1, "1 jour"],
  [7, "7 jours"],
  [30, "30 jours"],
  [90, "90 jours"],
  [365, "1 an"],
];

const ACCENTS = ["#a3e635", "#22c55e", "#06b6d4", "#3b82f6", "#8b5cf6", "#ec4899", "#f97316", "#f59e0b"];

const MODELS: Record<Exclude<AiProvider, "ollama">, string[]> = {
  anthropic: ["claude-opus-5", "claude-sonnet-5", "claude-haiku-4-5"],
  openai: ["gpt-5", "gpt-5-mini"],
};

interface Props {
  value: SettingsView;
  stats: Stats | null;
  onChange: (s: SettingsView) => void;
  onClose: () => void;
}

export function Settings({ value, stats, onChange, onClose }: Props) {
  const [tab, setTab] = useState<Tab>("general");
  const [draft, setDraft] = useState<SettingsView>(value);
  const [saved, setSaved] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useAutoClear(saved, setSaved, 1200);

  const save = async (patch: Partial<SettingsT> = {}) => {
    const next = { ...draft, ...patch };
    setDraft(next);
    try {
      const updated = await api.setSettings(next);
      setDraft(updated);
      onChange(updated);
      setError(null);
      setSaved("Enregistré");
    } catch (e) {
      setError(errorText(e));
      setDraft(value);
    }
  };
  // Text fields are edited locally and saved when they lose focus.
  const edit = (patch: Partial<SettingsT>) => setDraft((d) => ({ ...d, ...patch }));
  const text = (key: keyof SettingsT, placeholder?: string, mono = true) => (
    <input
      value={String(draft[key] ?? "")}
      placeholder={placeholder}
      spellCheck={false}
      onChange={(e) => edit({ [key]: e.target.value } as Partial<SettingsT>)}
      onBlur={() => draft[key] !== value[key] && save()}
      className={cn(inputClass, mono && "font-mono")}
    />
  );

  return (
    <Dialog title="Paramètres" onClose={onClose} width="w-[780px] h-[82vh]">
      <div className="flex h-full">
        <nav className="w-[170px] shrink-0 space-y-0.5 border-r border-ink-700/60 bg-ink-950/40 p-3">
          {TABS.map(([key, label, Icon]) => (
            <button
              key={key}
              onClick={() => setTab(key)}
              className={cn(
                "flex w-full items-center gap-2.5 rounded-lg px-3 py-2 text-left text-[13px]",
                tab === key ? "bg-ink-700/60 text-ink-50" : "text-ink-300 hover:bg-ink-800/60 hover:text-ink-100",
              )}
            >
              <Icon size={13} className={tab === key ? "text-accent" : undefined} />
              {label}
            </button>
          ))}
        </nav>

        <div className="min-w-0 flex-1 space-y-6 overflow-auto p-6">
          {(error || saved) && (
            <div
              className={cn(
                "flex items-center gap-1.5 rounded-md px-3 py-2 text-[12px]",
                error ? "border border-red-500/40 bg-red-500/10 text-red-400" : "text-accent",
              )}
            >
              {!error && <Check size={12} />} {error ?? saved}
            </div>
          )}

          {tab === "general" && (
            <>
              <Field
                label="Raccourci global"
                hint="Affiche ou masque Clipper depuis n'importe quelle application. Choisissez une combinaison libre (ex. Ctrl+Shift+V, Ctrl+Alt+C)."
              >
                <ShortcutCapture value={draft.shortcut} onChange={(shortcut) => save({ shortcut })} />
              </Field>
              <Row label="Lancer Clipper au démarrage de Windows" hint="Clipper démarre alors discrètement dans la zone de notification.">
                <Toggle label="Démarrage automatique" checked={draft.launch_at_startup} onChange={(v) => save({ launch_at_startup: v })} />
              </Row>
              <Row label="Suspendre la capture" hint="Plus rien n'est enregistré tant que la capture est suspendue.">
                <Toggle label="Suspendre la capture" checked={draft.monitor_paused} onChange={(v) => save({ monitor_paused: v })} />
              </Row>
            </>
          )}

          {tab === "capture" && (
            <>
              <p className="text-[12px] leading-relaxed text-ink-400">
                Clipper n'enregistre jamais ce que les gestionnaires de mots de passe marquent comme confidentiel
                (KeePass, Bitwarden, 1Password…), comme l'historique du presse-papiers de Windows.
              </p>
              <Field
                label="Applications ignorées"
                hint="Nom de l'exécutable, un par ligne (ex. keepassxc.exe). Le bouton « Ignorer » de l'aperçu ajoute l'application d'origine d'un élément."
              >
                <textarea
                  value={draft.ignore_apps.join("\n")}
                  onChange={(e) => edit({ ignore_apps: e.target.value.split("\n") })}
                  onBlur={() => save({ ignore_apps: draft.ignore_apps.map((a) => a.trim()).filter(Boolean) })}
                  rows={6}
                  spellCheck={false}
                  className={cn(inputClass, "h-auto py-2 font-mono")}
                />
              </Field>
            </>
          )}

          {tab === "retention" && (
            <>
              <Field
                label="Supprimer les éléments inutilisés depuis"
                hint={
                  draft.auto_delete_days === 0
                    ? "Rien n'est supprimé en fonction de l'âge."
                    : `Vérifié toutes les heures. Les éléments non utilisés depuis ${draft.auto_delete_days} jour(s) sont supprimés.`
                }
              >
                <div className="flex flex-wrap gap-1.5">
                  {RETENTION.map(([days, label]) => (
                    <Chip key={days} active={draft.auto_delete_days === days} onClick={() => save({ auto_delete_days: days })}>
                      {label}
                    </Chip>
                  ))}
                </div>
              </Field>
              <Row label="Conserver les éléments épinglés" hint="Ils échappent aussi à « Effacer l'historique ».">
                <Toggle label="Conserver les épinglés" checked={draft.keep_pinned} onChange={(v) => save({ keep_pinned: v })} />
              </Row>
              <Row label="Conserver les favoris">
                <Toggle label="Conserver les favoris" checked={draft.keep_favorites} onChange={(v) => save({ keep_favorites: v })} />
              </Row>
              <Field
                label="Nombre maximal d'éléments"
                hint="Au-delà, les éléments les moins récemment utilisés (hors épinglés) sont supprimés. 0 = illimité."
              >
                <input
                  type="number"
                  min={0}
                  step={500}
                  value={draft.max_items}
                  onChange={(e) => edit({ max_items: Math.max(0, parseInt(e.target.value) || 0) })}
                  onBlur={() => draft.max_items !== value.max_items && save()}
                  className={cn(inputClass, "w-32")}
                />
              </Field>
              <CleanupButton disabled={draft.auto_delete_days === 0} />
            </>
          )}

          {tab === "ai" && (
            <>
              <Field label="Fournisseur">
                <div className="flex gap-1.5">
                  {(["ollama", "anthropic", "openai"] as AiProvider[]).map((p) => (
                    <Chip key={p} active={draft.ai_provider === p} onClick={() => save({ ai_provider: p })}>
                      {p === "ollama" ? "Ollama (local)" : p === "anthropic" ? "Claude" : "OpenAI"}
                    </Chip>
                  ))}
                </div>
              </Field>
              <p className="text-[12px] leading-relaxed text-ink-400">
                {draft.ai_provider === "ollama"
                  ? "Le texte est traité sur cet ordinateur par Ollama (ollama.com). Rien ne quitte votre machine."
                  : "Le texte de l'élément choisi est envoyé au fournisseur uniquement lorsque vous cliquez sur une action IA. La clé est conservée dans le Gestionnaire d'identifiants de Windows."}
              </p>

              {draft.ai_provider === "ollama" && (
                <>
                  <Field label="Adresse d'Ollama">{text("ollama_url", "http://localhost:11434")}</Field>
                  <Field
                    label="Modèle"
                    hint={
                      <>
                        Installez-le avec <code>ollama pull {draft.ollama_model || "gemma3:4b"}</code>.
                      </>
                    }
                  >
                    {text("ollama_model", "gemma3:4b")}
                  </Field>
                </>
              )}

              {draft.ai_provider === "anthropic" && (
                <>
                  <ApiKeyField
                    provider="anthropic"
                    isSet={draft.anthropic_key_set}
                    placeholder="sk-ant-…"
                    onSaved={(set) => {
                      const next = { ...draft, anthropic_key_set: set };
                      setDraft(next);
                      onChange(next);
                    }}
                  />
                  <ModelField models={MODELS.anthropic} value={draft.anthropic_model} onPick={(m) => save({ anthropic_model: m })}>
                    {text("anthropic_model")}
                  </ModelField>
                </>
              )}

              {draft.ai_provider === "openai" && (
                <>
                  <ApiKeyField
                    provider="openai"
                    isSet={draft.openai_key_set}
                    placeholder="sk-…"
                    onSaved={(set) => {
                      const next = { ...draft, openai_key_set: set };
                      setDraft(next);
                      onChange(next);
                    }}
                  />
                  <ModelField models={MODELS.openai} value={draft.openai_model} onPick={(m) => save({ openai_model: m })}>
                    {text("openai_model")}
                  </ModelField>
                  <Field label="URL de l'API" hint="Pour un service compatible OpenAI. HTTPS obligatoire hors de cet ordinateur.">
                    {text("openai_base_url", "https://api.openai.com")}
                  </Field>
                </>
              )}
            </>
          )}

          {tab === "appearance" && (
            <>
              <Field label="Thème">
                <div className="flex gap-1.5">
                  {(
                    [
                      ["auto", "Système"],
                      ["dark", "Sombre"],
                      ["light", "Clair"],
                    ] as const
                  ).map(([t, label]) => (
                    <Chip key={t} active={draft.theme === t} onClick={() => save({ theme: t })}>
                      {label}
                    </Chip>
                  ))}
                </div>
              </Field>
              <Field label="Couleur d'accent">
                <div className="flex flex-wrap items-center gap-2">
                  {ACCENTS.map((hex) => (
                    <button
                      key={hex}
                      onClick={() => save({ accent_color: hex })}
                      aria-label={`Accent ${hex}`}
                      className={cn(
                        "h-8 w-8 rounded-full border-2",
                        draft.accent_color.toLowerCase() === hex ? "border-ink-50" : "border-transparent",
                      )}
                      style={{ backgroundColor: hex }}
                    />
                  ))}
                  <input
                    type="color"
                    value={draft.accent_color}
                    onChange={(e) => edit({ accent_color: e.target.value })}
                    onBlur={() => draft.accent_color !== value.accent_color && save()}
                    aria-label="Couleur personnalisée"
                    className="h-8 w-12 cursor-pointer rounded border border-ink-700 bg-ink-800"
                  />
                </div>
              </Field>
              <Field label="Densité de la liste">
                <div className="flex gap-1.5">
                  <Chip active={draft.density === "comfortable"} onClick={() => save({ density: "comfortable" })}>
                    Confortable
                  </Chip>
                  <Chip active={draft.density === "compact"} onClick={() => save({ density: "compact" })}>
                    Compacte
                  </Chip>
                </div>
              </Field>
            </>
          )}

          {tab === "data" && <DataTab stats={stats} keepPinned={draft.keep_pinned} dataDir={draft.data_dir} />}

          {tab === "about" && (
            <div className="space-y-4 text-[12.5px] leading-relaxed text-ink-300">
              <div className="flex items-center gap-3">
                <img src="/clipper.svg" alt="" className="h-10 w-10" />
                <div>
                  <div className="font-display text-[16px] font-semibold text-ink-50">Clipper</div>
                  <div className="text-[11.5px] text-ink-500">Version {__APP_VERSION__} · licence MIT</div>
                </div>
              </div>
              <p>
                Votre historique est stocké uniquement sur cet ordinateur, dans <code className="text-ink-100">{draft.data_dir}</code>.
                Clipper n'envoie aucune donnée de télémétrie. Seules les actions IA avec OpenAI ou Claude transmettent,
                à votre demande, le texte de l'élément concerné.
              </p>
              <p>
                Code source :{" "}
                <span className="font-mono text-ink-100">github.com/titilyonnais/Clipper</span>
              </p>
            </div>
          )}
        </div>
      </div>
    </Dialog>
  );
}

function Row({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-6">
      <div>
        <div className="text-[12.5px] font-medium text-ink-100">{label}</div>
        {hint && <div className="mt-0.5 text-[11px] text-ink-500">{hint}</div>}
      </div>
      {children}
    </div>
  );
}

function ModelField({
  models,
  value,
  onPick,
  children,
}: {
  models: string[];
  value: string;
  onPick: (m: string) => void;
  children: React.ReactNode;
}) {
  return (
    <Field label="Modèle">
      <div className="mb-2 flex flex-wrap gap-1.5">
        {models.map((m) => (
          <Chip key={m} active={value === m} onClick={() => onPick(m)} className="h-7 font-mono text-[11px]">
            {m}
          </Chip>
        ))}
      </div>
      {children}
    </Field>
  );
}

function ApiKeyField({
  provider,
  isSet,
  placeholder,
  onSaved,
}: {
  provider: "openai" | "anthropic";
  isSet: boolean;
  placeholder: string;
  onSaved: (isSet: boolean) => void;
}) {
  const [key, setKey] = useState("");
  const [error, setError] = useState<string | null>(null);
  const store = async (value: string) => {
    try {
      onSaved(await api.setApiKey(provider, value));
      setKey("");
      setError(null);
    } catch (e) {
      setError(errorText(e));
    }
  };
  return (
    <Field
      label="Clé API"
      hint={error ?? (isSet ? "Une clé est enregistrée. Saisissez-en une nouvelle pour la remplacer." : undefined)}
    >
      <div className="flex gap-2">
        <input
          type="password"
          value={key}
          autoComplete="off"
          placeholder={isSet ? "••••••••••••" : placeholder}
          onChange={(e) => setKey(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && key.trim() && store(key)}
          className={cn(inputClass, "font-mono")}
        />
        <button className={buttonClass} disabled={!key.trim()} onClick={() => store(key)}>
          Enregistrer
        </button>
        {isSet && (
          <button className={cn(buttonClass, "hover:text-red-400")} onClick={() => store("")}>
            Supprimer
          </button>
        )}
      </div>
    </Field>
  );
}

function CleanupButton({ disabled }: { disabled: boolean }) {
  const [result, setResult] = useState<string | null>(null);
  useAutoClear(result, setResult);
  return (
    <div className="flex items-center gap-3">
      <button
        className={buttonClass}
        disabled={disabled}
        onClick={async () => setResult(`${await api.cleanupNow()} élément(s) supprimé(s).`)}
      >
        <Trash2 size={12} /> Appliquer maintenant
      </button>
      {result && <span className="text-[11.5px] text-accent">{result}</span>}
    </div>
  );
}

function DataTab({ stats, keepPinned, dataDir }: { stats: Stats | null; keepPinned: boolean; dataDir: string }) {
  const [message, setMessage] = useState<string | null>(null);
  const [confirming, setConfirming] = useState(false);
  useAutoClear(message, setMessage, 5000);

  const guard = (fn: () => Promise<string | null>) => () =>
    fn()
      .then((m) => m && setMessage(m))
      .catch((e) => setMessage(`Erreur : ${errorText(e)}`));

  return (
    <>
      {stats && (
        <div className="grid grid-cols-3 gap-3 rounded-xl border border-ink-700/60 bg-ink-800/40 p-4">
          <Stat label="Éléments" value={stats.total.toLocaleString("fr-FR")} />
          <Stat label="Épinglés" value={stats.pinned.toLocaleString("fr-FR")} />
          <Stat label="Espace disque" value={humanBytes(stats.disk_bytes)} />
          <div className="col-span-3 truncate font-mono text-[11px] text-ink-500" title={dataDir}>
            {dataDir}
          </div>
        </div>
      )}

      <Field label="Exporter / importer" hint="Fichier JSON (images incluses). L'import ignore les éléments déjà présents.">
        <div className="flex gap-2">
          <button
            className={buttonClass}
            onClick={guard(() => api.exportHistory())}
          >
            <Download size={12} /> Exporter
          </button>
          <button
            className={buttonClass}
            onClick={guard(async () => {
              const r = await api.importHistory();
              return r && `${r.imported} élément(s) importé(s), ${r.skipped} déjà présent(s) ou invalide(s).`;
            })}
          >
            <Upload size={12} /> Importer
          </button>
        </div>
      </Field>

      {message && <p className="text-[12px] text-accent">{message}</p>}

      <div className="rounded-xl border border-red-500/50 bg-red-500/[0.06] p-4">
        <div className="mb-1 text-[13px] font-semibold text-red-400">Effacer l'historique</div>
        <p className="mb-3 text-[12px] text-ink-300">
          Suppression définitive de tous les éléments{keepPinned ? ", sauf les épinglés" : ", épinglés compris"}.
        </p>
        {confirming ? (
          <div className="flex gap-2">
            <button className={buttonClass} onClick={() => setConfirming(false)}>
              Annuler
            </button>
            <button
              className="h-9 rounded-md bg-red-600 px-4 text-[12.5px] font-semibold text-white hover:bg-red-700"
              onClick={guard(async () => {
                setConfirming(false);
                return `${await api.clearHistory()} élément(s) supprimé(s).`;
              })}
            >
              Oui, tout effacer
            </button>
          </div>
        ) : (
          <button
            className="inline-flex h-9 items-center gap-1.5 rounded-md bg-red-600 px-4 text-[12.5px] font-semibold text-white hover:bg-red-700"
            onClick={() => setConfirming(true)}
          >
            <Trash2 size={12} /> Effacer l'historique…
          </button>
        )}
      </div>
    </>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <div className="text-[10.5px] uppercase tracking-wide text-ink-500">{label}</div>
      <div className="font-display text-[18px] font-semibold tabular-nums text-ink-50">{value}</div>
    </div>
  );
}
