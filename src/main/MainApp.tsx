import { useCallback, useEffect, useRef, useState } from "react";
import { api } from "@/lib/api";
import { useTauriEvent } from "@/lib/hooks";
import { useSettings } from "@/lib/settings";
import { useAiOnline } from "@/lib/ai";
import { run } from "@/clip/actions";
import { Button } from "@/ui/button";
import { Dialog } from "@/ui/dialog";
import { Input } from "@/ui/form";
import { TitleBar } from "./TitleBar";
import { Sidebar, type View } from "./Sidebar";
import { HistoryView, viewTitle, type HistoryHandle } from "./HistoryView";
import { SnippetsView, useSnippets } from "./SnippetsView";
import { SettingsView } from "./SettingsView";
import { Onboarding } from "./Onboarding";
import type { Collection, SourceApp, Stats } from "@/types";

type NameDialog = { mode: "create"; assign?: number[] } | { mode: "rename"; collection: Collection };

export function MainApp() {
  const { settings } = useSettings();
  const [view, setView] = useState<View>({ kind: "history" });
  const [stats, setStats] = useState<Stats | null>(null);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [apps, setApps] = useState<SourceApp[]>([]);
  const [nameDialog, setNameDialog] = useState<NameDialog | null>(null);
  const [onboardingDone, setOnboardingDone] = useState(false);
  const history = useRef<HistoryHandle>(null);
  const snippets = useSnippets();
  const aiOnline = useAiOnline(settings);

  const refresh = useCallback(() => {
    api.stats().then(setStats);
    api.collections().then(setCollections);
    api.sourceApps().then(setApps);
  }, []);
  useEffect(refresh, [refresh]);
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useTauriEvent("clips:changed", () => {
    clearTimeout(timer.current);
    timer.current = setTimeout(refresh, 120);
  });
  useTauriEvent("window:shown", () => {
    if (view.kind !== "settings" && view.kind !== "snippets") history.current?.focusSearch();
  });

  // A deleted collection or an app with no clips left: back to the history.
  useEffect(() => {
    if (view.kind === "collection" && !collections.some((c) => c.id === view.id)) setView({ kind: "history" });
  }, [collections, view]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.key.toLowerCase() === "f" && !e.defaultPrevented) {
        e.preventDefault();
        if (view.kind === "settings" || view.kind === "snippets") setView({ kind: "history" });
        setTimeout(() => history.current?.focusSearch());
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [view]);

  const showHistory = view.kind !== "settings" && view.kind !== "snippets";

  return (
    <div className="flex h-screen flex-col bg-background">
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        <Sidebar
          view={view}
          onView={setView}
          stats={stats}
          collections={collections}
          apps={apps}
          snippetCount={snippets.length}
          onNewCollection={() => setNameDialog({ mode: "create" })}
          onRenameCollection={(c) => setNameDialog({ mode: "rename", collection: c })}
        />
        {showHistory && (
          <HistoryView
            key={JSON.stringify(view)}
            ref={history}
            view={view}
            title={viewTitle(view, collections)}
            collections={collections}
            onNewCollection={(assign) => setNameDialog({ mode: "create", assign })}
            aiOnline={aiOnline}
          />
        )}
        {view.kind === "snippets" && <SnippetsView />}
        {view.kind === "settings" && <SettingsView stats={stats} />}
      </div>

      {nameDialog && <CollectionNameDialog dialog={nameDialog} onClose={() => setNameDialog(null)} />}
      {settings && !settings.onboarded && !onboardingDone && <Onboarding onDone={() => setOnboardingDone(true)} />}
    </div>
  );
}

function CollectionNameDialog({ dialog, onClose }: { dialog: NameDialog; onClose: () => void }) {
  const [name, setName] = useState(dialog.mode === "rename" ? dialog.collection.name : "");
  const submit = () =>
    run(async () => {
      if (dialog.mode === "rename") {
        await api.renameCollection(dialog.collection.id, name);
      } else {
        const id = await api.createCollection(name);
        if (dialog.assign?.length) await api.setCollection(dialog.assign, id);
      }
      onClose();
    });
  return (
    <Dialog
      title={dialog.mode === "rename" ? "Renommer la collection" : "Nouvelle collection"}
      description={dialog.mode === "create" ? "Les éléments rangés dans une collection ne sont jamais supprimés automatiquement." : undefined}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Annuler</Button>
          <Button variant="primary" disabled={!name.trim()} onClick={submit}>
            {dialog.mode === "rename" ? "Renommer" : "Créer"}
          </Button>
        </>
      }
    >
      <Input
        autoFocus
        value={name}
        maxLength={48}
        placeholder="Travail, Code, Recettes…"
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && name.trim() && submit()}
        className="h-9"
      />
    </Dialog>
  );
}
