import { useEffect, useRef, useState } from "react";
import { Check, Download, FolderOpen, RotateCcw, Upload } from "lucide-react";
import { api, errorText } from "@/lib/api";
import { useSettings } from "@/lib/settings";
import { cn, fullDate, humanBytes, plural } from "@/lib/utils";
import { run } from "@/clip/actions";
import { Button } from "@/ui/button";
import { Dialog } from "@/ui/dialog";
import { Field, Input, Row, Segmented, Switch, Textarea, useSlidingThumb } from "@/ui/form";
import { Badge, Kbd, Logo } from "@/ui/misc";
import { toast } from "@/ui/toast";
import { ShortcutCapture } from "./ShortcutCapture";
import type { AiProvider, BackupInfo, Settings, Stats } from "@/types";

declare const __APP_VERSION__: string;

type Tab = "shortcut" | "capture" | "storage" | "ai" | "appearance" | "about";
const TABS: [Tab, string][] = [
  ["shortcut", "Raccourci et collage"],
  ["capture", "Capture"],
  ["storage", "Stockage"],
  ["ai", "Intelligence artificielle"],
  ["appearance", "Apparence"],
  ["about", "À propos"],
];

export function SettingsView({ stats }: { stats: Stats | null }) {
  const [tab, setTab] = useState<Tab>("shortcut");
  const { settings } = useSettings();
  const navRef = useRef<HTMLElement>(null);
  const thumb = useSlidingThumb(navRef, tab, '[aria-current="page"]');
  if (!settings) return null;
  return (
    <div className="flex min-w-0 flex-1 animate-in">
      <nav ref={navRef} className="relative w-60 shrink-0 space-y-px border-r border-line px-2.5 py-3" aria-label="Rubriques">
        {thumb && (
          <span
            aria-hidden="true"
            className={cn(
              "absolute inset-x-2.5 top-0 rounded-ctl bg-selected",
              thumb.animate && "transition-transform duration-200 ease-out-soft",
            )}
            style={{ height: thumb.height, transform: `translateY(${thumb.top}px)` }}
          />
        )}
        {TABS.map(([key, label]) => (
          <button
            key={key}
            type="button"
            onClick={() => setTab(key)}
            aria-current={tab === key ? "page" : undefined}
            className={cn(
              "relative flex h-8 w-full items-center rounded-ctl px-2.5 text-left text-sm transition-colors duration-150",
              tab === key ? "text-foreground" : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
            )}
          >
            {label}
          </button>
        ))}
      </nav>
      <div className="min-w-0 flex-1 overflow-y-auto">
        <div key={tab} className="mx-auto max-w-2xl animate-rise px-8 py-8">
          <h1 className="mb-6 text-xl text-foreground">{TABS.find(([k]) => k === tab)?.[1]}</h1>
          {tab === "shortcut" && <ShortcutSection />}
          {tab === "capture" && <CaptureSection />}
          {tab === "storage" && <StorageSection stats={stats} />}
          {tab === "ai" && <AiSection />}
          {tab === "appearance" && <AppearanceSection />}
          {tab === "about" && <AboutSection />}
        </div>
      </div>
    </div>
  );
}

/** Save a change and report errors. */
function useSave() {
  const { update } = useSettings();
  return async (patch: Partial<Settings>) => {
    const error = await update(patch);
    if (error) toast(error, true);
  };
}

function Group({ title, children }: { title?: string; children: React.ReactNode }) {
  return (
    <section className="mb-8">
      {title && <h2 className="mb-1 text-sm text-muted-foreground">{title}</h2>}
      <div className="divide-y divide-line rounded-card border border-line bg-surface px-4">{children}</div>
    </section>
  );
}

function ShortcutSection() {
  const { settings, reload } = useSettings();
  const save = useSave();
  const [busy, setBusy] = useState(false);
  if (!settings) return null;
  const winV = settings.shortcut_mode === "win_v";

  const enable = async (restart: boolean) => {
    setBusy(true);
    try {
      const ok = await api.enableWinV(restart);
      toast(ok ? "Win+V ouvre maintenant Clipper." : "Relancez l'Explorateur pour libérer Win+V.", !ok);
    } catch (e) {
      toast(errorText(e), true);
    } finally {
      setBusy(false);
      reload();
    }
  };
  const disable = async () => {
    setBusy(true);
    try {
      await api.disableWinV(true);
      toast("Win+V est rendu à Windows.");
    } catch (e) {
      toast(errorText(e), true);
    } finally {
      setBusy(false);
      reload();
    }
  };

  return (
    <>
      <Group>
        <div className="py-4">
          <div className="flex items-center justify-between gap-8">
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2 text-sm whitespace-nowrap text-foreground">
                Ouvrir Clipper avec <Kbd>Win</Kbd>
                <Kbd>V</Kbd>
                {winV && (settings.win_v_active ? <Badge tone="ok">Actif</Badge> : <Badge tone="warn">Explorateur à relancer</Badge>)}
              </div>
              <p className="mt-1 text-13 leading-relaxed text-muted-foreground">
                Remplace l'historique du presse-papiers de Windows. Clipper utilise le réglage officiel de l'Explorateur, qui doit
                redémarrer une fois (la barre des tâches disparaît deux secondes). Aucune surveillance du clavier.
              </p>
            </div>
            <div className="flex shrink-0 gap-2">
              {!winV && (
                <Button variant="primary" disabled={busy} onClick={() => enable(true)}>
                  Activer
                </Button>
              )}
              {winV && !settings.win_v_active && (
                <Button variant="primary" disabled={busy} onClick={() => enable(true)}>
                  Relancer l'Explorateur
                </Button>
              )}
              {winV && (
                <Button disabled={busy} onClick={disable}>
                  Désactiver
                </Button>
              )}
            </div>
          </div>
        </div>
        {!winV && (
          <Row label="Raccourci" hint="Combinaison qui ouvre le collage rapide depuis n'importe quelle application.">
            <ShortcutCapture value={settings.shortcut} onChange={(shortcut) => save({ shortcut })} />
          </Row>
        )}
      </Group>
      <Group title="Collage">
        <Row
          label="Coller directement"
          hint="Entrée dans le collage rapide colle dans l'application d'où vous venez. Sinon, l'élément est seulement copié."
        >
          <Switch label="Coller directement" checked={settings.paste_directly} onChange={(v) => save({ paste_directly: v })} />
        </Row>
        <Row label="Toujours coller en texte brut" hint="Ignore la mise en forme (gras, liens, tableaux). Maj+Entrée le fait ponctuellement.">
          <Switch label="Toujours en texte brut" checked={settings.always_plain_text} onChange={(v) => save({ always_plain_text: v })} />
        </Row>
        <Row label="Démarrer avec Windows" hint="Clipper démarre discrètement dans la zone de notification.">
          <Switch label="Démarrer avec Windows" checked={settings.launch_at_startup} onChange={(v) => save({ launch_at_startup: v })} />
        </Row>
      </Group>
      <Group title="Dans le collage rapide">
        <ShortcutHelp />
      </Group>
    </>
  );
}

export function ShortcutHelp() {
  const rows: [string[], string][] = [
    [["Entrée"], "Coller l'élément sélectionné"],
    [["Maj", "Entrée"], "Coller en texte brut"],
    [["Ctrl", "1…9"], "Coller l'un des neuf premiers"],
    [["Ctrl", "E"], "Modifier avant de coller"],
    [["Maj", "↑↓"], "Sélection pour un collage en série"],
    [["Tab"], "Historique, snippets, collections"],
    [["Ctrl", "P"], "Épingler"],
    [["Ctrl", "O"], "Ouvrir la grande fenêtre"],
    [["Ctrl", "Suppr"], "Supprimer"],
    [["Échap"], "Fermer"],
  ];
  return (
    <div className="grid grid-cols-2 gap-x-6 gap-y-2.5 py-4">
      {rows.map(([keys, label]) => (
        <div key={label} className="flex items-center justify-between gap-3 text-13">
          <span className="text-muted-foreground">{label}</span>
          <span className="flex gap-1">
            {keys.map((k) => (
              <Kbd key={k}>{k}</Kbd>
            ))}
          </span>
        </div>
      ))}
    </div>
  );
}

function CaptureSection() {
  const { settings } = useSettings();
  const save = useSave();
  const [apps, setApps] = useState(settings?.ignore_apps.join("\n") ?? "");
  if (!settings) return null;
  const paused = settings.paused_until;
  return (
    <>
      <Group>
        <div className="py-4">
          <div className="text-sm text-foreground">Mode incognito</div>
          <p className="mt-1 mb-3 text-13 text-muted-foreground">
            {paused === "forever"
              ? "La capture est suspendue jusqu'à ce que vous la repreniez."
              : paused
                ? `Rien n'est enregistré jusqu'à ${new Date(paused).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" })}.`
                : "Suspendez l'enregistrement le temps de manipuler des données privées."}
          </p>
          <div className="flex flex-wrap gap-2">
            {paused ? (
              <Button variant="primary" onClick={() => api.setIncognito(null)}>
                Reprendre la capture
              </Button>
            ) : (
              <>
                <Button onClick={() => api.setIncognito(5)}>5 minutes</Button>
                <Button onClick={() => api.setIncognito(60)}>1 heure</Button>
                <Button onClick={() => api.setIncognito(0)}>Jusqu'à réactivation</Button>
              </>
            )}
          </div>
        </div>
      </Group>
      <Group title="Contenu">
        <Row
          label="Masquer les secrets"
          hint="Mots de passe, clés d'API, jetons et cartes bancaires sont masqués, exclus de la recherche et jamais envoyés à une IA en ligne."
        >
          <Switch label="Masquer les secrets" checked={settings.detect_secrets} onChange={(v) => save({ detect_secrets: v })} />
        </Row>
        <Row label="Conserver la mise en forme" hint="Garde le gras, les liens et les tableaux copiés depuis Word, Excel ou le web.">
          <Switch label="Conserver la mise en forme" checked={settings.keep_rich_text} onChange={(v) => save({ keep_rich_text: v })} />
        </Row>
        <Row label="Reconnaître le texte des images" hint="Avec le moteur OCR intégré à Windows, hors ligne. Le texte des captures devient cherchable.">
          <Switch label="OCR" checked={settings.ocr_enabled} onChange={(v) => save({ ocr_enabled: v })} />
        </Row>
      </Group>
      <Group title="Applications ignorées">
        <Field
          label="Rien de ce qui est copié depuis ces applications n'est enregistré"
          hint="Un nom d'exécutable par ligne (ex. keepassxc.exe). Les gestionnaires de mots de passe qui le signalent sont ignorés automatiquement."
        >
          <Textarea
            rows={5}
            value={apps}
            onChange={(e) => setApps(e.target.value)}
            onBlur={() => save({ ignore_apps: apps.split("\n").map((a) => a.trim()).filter(Boolean) })}
            className="font-mono text-13"
          />
        </Field>
      </Group>
    </>
  );
}

function StorageSection({ stats }: { stats: Stats | null }) {
  const { settings } = useSettings();
  const save = useSave();
  const [max, setMax] = useState(String(settings?.max_items ?? 0));
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [confirm, setConfirm] = useState<"clear" | BackupInfo | null>(null);
  useEffect(() => {
    api.backups().then(setBackups);
  }, []);
  if (!settings) return null;

  return (
    <>
      {stats && (
        <div className="mb-8 grid grid-cols-3 gap-3">
          {[
            ["Éléments", stats.total.toLocaleString("fr-FR")],
            ["Épinglés", stats.pinned.toLocaleString("fr-FR")],
            ["Sur le disque", humanBytes(stats.disk_bytes)],
          ].map(([label, value]) => (
            <div key={label} className="rounded-card border border-line bg-surface px-4 py-3">
              <div className="text-xs text-subtle-foreground">{label}</div>
              <div className="tabular mt-1 text-xl text-foreground">{value}</div>
            </div>
          ))}
        </div>
      )}
      <Group title="Conservation">
        <Row label="Nombre maximal d'éléments" hint="Au-delà, les plus anciens sont supprimés. Les épinglés et les collections sont toujours gardés. 0 = illimité.">
          <Input
            type="number"
            min={0}
            step={500}
            value={max}
            onChange={(e) => setMax(e.target.value)}
            onBlur={() => save({ max_items: Math.max(0, parseInt(max) || 0) })}
            className="w-28 text-right tabular"
          />
        </Row>
        <Row label="Supprimer les éléments inutilisés depuis">
          <Segmented
            label="Durée de conservation"
            value={settings.auto_delete_days}
            onChange={(v) => save({ auto_delete_days: v })}
            options={[
              { value: 0, label: "Jamais" },
              { value: 7, label: "7 j" },
              { value: 30, label: "30 j" },
              { value: 90, label: "90 j" },
              { value: 365, label: "1 an" },
            ]}
          />
        </Row>
      </Group>
      <Group title="Sauvegardes">
        <Row label="Sauvegarde quotidienne" hint="Copie de la base chaque jour ; les sept dernières sont gardées.">
          <Switch label="Sauvegarde quotidienne" checked={settings.backups_enabled} onChange={(v) => save({ backups_enabled: v })} />
        </Row>
        <div className="py-3">
          <div className="mb-2 flex min-h-8 items-center justify-between">
            <span className="text-sm text-foreground">Restaurer</span>
            <Button variant="ghost" className="-mr-2.5" onClick={() => run(async () => setBackups(await api.backupNow()), "Sauvegarde créée.")}>
              Sauvegarder maintenant
            </Button>
          </div>
          {backups.length ? (
            <ul className="-mx-2 space-y-0.5">
              {backups.map((b) => (
                <li key={b.name} className="flex h-9 items-center justify-between gap-3 rounded-ctl pr-1 pl-2 text-13 transition-colors hover:bg-muted/60">
                  <span className="text-foreground">{fullDate(b.created_at)}</span>
                  <span className="ml-auto text-xs text-subtle-foreground">{humanBytes(b.size)}</span>
                  <Button size="xs" variant="ghost" onClick={() => setConfirm(b)}>
                    <RotateCcw /> Restaurer
                  </Button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-13 text-subtle-foreground">Aucune sauvegarde pour l'instant.</p>
          )}
        </div>
      </Group>
      <Group title="Données">
        <Row label="Exporter ou importer l'historique" hint="Fichier JSON, images et collections compris. L'import ignore ce qui existe déjà.">
          <Button onClick={() => run(async () => { const m = await api.exportHistory(); if (m) toast(m); })}>
            <Download /> Exporter
          </Button>
          <Button
            onClick={() =>
              run(async () => {
                const r = await api.importHistory();
                if (r) toast(`${plural(r.imported, "élément importé", "éléments importés")}, ${r.skipped} déjà présent(s).`);
              })
            }
          >
            <Upload /> Importer
          </Button>
        </Row>
        <Row label="Dossier des données" hint={<span className="font-mono text-xs break-all">{settings.data_dir}</span>}>
          <Button onClick={() => run(() => api.openDataFolder())}>
            <FolderOpen /> Ouvrir
          </Button>
        </Row>
        <Row label="Effacer l'historique" hint="Définitif. Les épinglés, les collections et les snippets sont conservés.">
          <Button variant="danger" onClick={() => setConfirm("clear")}>
            Effacer…
          </Button>
        </Row>
      </Group>

      {confirm === "clear" && (
        <Dialog
          title="Effacer l'historique ?"
          description="Tous les éléments qui ne sont ni épinglés ni rangés dans une collection seront supprimés définitivement."
          onClose={() => setConfirm(null)}
          footer={
            <>
              <Button onClick={() => setConfirm(null)}>Annuler</Button>
              <Button
                variant="danger"
                onClick={() => {
                  setConfirm(null);
                  run(async () => toast(`${plural(await api.clearHistory(), "élément supprimé", "éléments supprimés")}.`));
                }}
              >
                Effacer
              </Button>
            </>
          }
        />
      )}
      {confirm && confirm !== "clear" && (
        <Dialog
          title="Restaurer cette sauvegarde ?"
          description={`L'historique revient à son état du ${fullDate(confirm.created_at)}. L'état actuel est d'abord sauvegardé.`}
          onClose={() => setConfirm(null)}
          footer={
            <>
              <Button onClick={() => setConfirm(null)}>Annuler</Button>
              <Button
                variant="primary"
                onClick={() => {
                  const name = confirm.name;
                  setConfirm(null);
                  run(async () => {
                    await api.restoreBackup(name);
                    setBackups(await api.backups());
                  }, "Sauvegarde restaurée.");
                }}
              >
                Restaurer
              </Button>
            </>
          }
        />
      )}
    </>
  );
}

const MODELS: Record<Exclude<AiProvider, "ollama">, string[]> = {
  anthropic: ["claude-opus-5", "claude-sonnet-5", "claude-haiku-4-5"],
  openai: ["gpt-5", "gpt-5-mini"],
};

function AiSection() {
  const { settings } = useSettings();
  const save = useSave();
  const [draft, setDraft] = useState<Partial<Settings>>({});
  if (!settings) return null;
  const value = <K extends keyof Settings>(k: K) => (draft[k] ?? settings[k]) as Settings[K];
  const text = (k: "ollama_url" | "ollama_model" | "openai_base_url" | "openai_model" | "anthropic_model", placeholder: string) => (
    <Input
      value={value(k)}
      placeholder={placeholder}
      onChange={(e) => setDraft({ ...draft, [k]: e.target.value })}
      onBlur={() => draft[k] !== undefined && draft[k] !== settings[k] && save({ [k]: draft[k] })}
      className="font-mono text-13"
    />
  );
  const provider = settings.ai_provider;

  return (
    <>
      <Group>
        <Row label="Fournisseur">
          <Segmented
            label="Fournisseur"
            value={provider}
            onChange={(v) => save({ ai_provider: v })}
            options={[
              { value: "ollama", label: "Ollama (local)" },
              { value: "anthropic", label: "Claude" },
              { value: "openai", label: "OpenAI" },
            ]}
          />
        </Row>
        <p className="py-3 text-13 leading-relaxed text-muted-foreground">
          {provider === "ollama"
            ? "Le texte est traité sur cet ordinateur par Ollama (ollama.com). Rien ne quitte votre machine."
            : "Le texte de l'élément est envoyé au fournisseur uniquement quand vous lancez une action IA, jamais s'il contient un secret. La clé reste dans le Gestionnaire d'identifiants de Windows."}
        </p>
      </Group>
      <Group title="Configuration">
        {provider === "ollama" ? (
          <>
            <Field label="Adresse d'Ollama">{text("ollama_url", "http://localhost:11434")}</Field>
            <Field label="Modèle" hint={<>Installez-le avec <code className="text-foreground">ollama pull {settings.ollama_model}</code>.</>}>
              {text("ollama_model", "gemma3:4b")}
            </Field>
          </>
        ) : (
          <>
            <ApiKey provider={provider} isSet={provider === "anthropic" ? settings.anthropic_key_set : settings.openai_key_set} />
            <Field label="Modèle">
              <div className="mb-2 flex flex-wrap gap-1.5">
                {MODELS[provider].map((m) => (
                  <Button
                    key={m}
                    size="xs"
                    variant={value(provider === "anthropic" ? "anthropic_model" : "openai_model") === m ? "primary" : "outline"}
                    onClick={() => save(provider === "anthropic" ? { anthropic_model: m } : { openai_model: m })}
                  >
                    <span className="font-mono">{m}</span>
                  </Button>
                ))}
              </div>
              {text(provider === "anthropic" ? "anthropic_model" : "openai_model", "")}
            </Field>
            {provider === "openai" && (
              <Field label="Adresse de l'API" hint="Pour un service compatible OpenAI. HTTPS obligatoire hors de cet ordinateur.">
                {text("openai_base_url", "https://api.openai.com")}
              </Field>
            )}
          </>
        )}
      </Group>
    </>
  );
}

function ApiKey({ provider, isSet }: { provider: "openai" | "anthropic"; isSet: boolean }) {
  const { reload } = useSettings();
  const [key, setKey] = useState("");
  const store = (value: string) =>
    run(async () => {
      await api.setApiKey(provider, value);
      setKey("");
      reload();
    }, value ? "Clé enregistrée." : "Clé supprimée.");
  return (
    <Field label="Clé d'API" hint={isSet ? "Une clé est enregistrée. Saisissez-en une nouvelle pour la remplacer." : undefined}>
      <div className="flex gap-2">
        <Input
          type="password"
          autoComplete="off"
          value={key}
          placeholder={isSet ? "••••••••••••••••" : provider === "anthropic" ? "sk-ant-…" : "sk-…"}
          onChange={(e) => setKey(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && key.trim() && store(key)}
          className="font-mono text-13"
        />
        <Button disabled={!key.trim()} onClick={() => store(key)}>
          {isSet ? "Remplacer" : "Enregistrer"}
        </Button>
        {isSet && (
          <Button variant="ghost" onClick={() => store("")}>
            Supprimer
          </Button>
        )}
      </div>
    </Field>
  );
}

function AppearanceSection() {
  const { settings } = useSettings();
  const save = useSave();
  if (!settings) return null;
  return (
    <Group>
      <Row label="Thème" hint="Noir par défaut. « Système » suit le mode clair ou sombre de Windows.">
        <Segmented
          label="Thème"
          value={settings.theme}
          onChange={(v) => save({ theme: v })}
          options={[
            { value: "auto", label: "Système" },
            { value: "dark", label: "Sombre" },
            { value: "light", label: "Clair" },
          ]}
        />
      </Row>
      <Row label="Densité de la liste">
        <Segmented
          label="Densité"
          value={settings.density}
          onChange={(v) => save({ density: v })}
          options={[
            { value: "comfortable", label: "Aérée" },
            { value: "compact", label: "Compacte" },
          ]}
        />
      </Row>
    </Group>
  );
}

function AboutSection() {
  const { settings } = useSettings();
  return (
    <div className="space-y-6">
      <div className="flex items-center gap-4">
        <Logo className="size-12 text-foreground" />
        <div>
          <div className="text-lg text-foreground">Clipper</div>
          <div className="text-13 text-muted-foreground">Version {__APP_VERSION__} · licence MIT</div>
        </div>
      </div>
      <Group>
        {[
          "Tout est stocké sur cet ordinateur, rien n'est envoyé ailleurs sans action de votre part.",
          "Aucune télémétrie, aucune publicité, aucun compte.",
          "Les secrets détectés ne quittent jamais la machine.",
          "Aucune surveillance du clavier : le collage direct et Win+V utilisent les mécanismes prévus par Windows.",
        ].map((t) => (
          <p key={t} className="flex gap-3 py-3 text-13 text-muted-foreground">
            <Check className="mt-0.5 size-4 shrink-0 text-foreground" /> {t}
          </p>
        ))}
      </Group>
      <p className="text-13 text-subtle-foreground">
        Code source : github.com/titilyonnais/Clipper · Données : <span className="font-mono">{settings?.data_dir}</span>
      </p>
    </div>
  );
}
