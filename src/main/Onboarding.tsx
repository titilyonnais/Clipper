import { useState } from "react";
import { ArrowRight, Check } from "lucide-react";
import { api, errorText } from "@/lib/api";
import { useSettings } from "@/lib/settings";
import { cn } from "@/lib/utils";
import { Button } from "@/ui/button";
import { Switch } from "@/ui/form";
import { Kbd, Logo, Spinner } from "@/ui/misc";

/** Three short steps on first launch: shortcut, paste behaviour, start-up. */
export function Onboarding({ onDone }: { onDone: () => void }) {
  const { settings, update, reload } = useSettings();
  const [step, setStep] = useState(0);
  const [busy, setBusy] = useState(false);
  const [winVError, setWinVError] = useState<string | null>(null);
  if (!settings) return null;

  const enableWinV = async () => {
    setBusy(true);
    setWinVError(null);
    try {
      const ok = await api.enableWinV(true);
      if (!ok) setWinVError("L'Explorateur n'a pas encore libéré Win+V. Réessayez depuis les paramètres, ou redémarrez la session.");
      reload();
      if (ok) setStep(1);
    } catch (e) {
      setWinVError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const finish = async () => {
    await api.completeOnboarding();
    reload();
    onDone();
  };

  const steps = [
    <Step
      key="shortcut"
      title="Ouvrez Clipper avec Win+V"
      text="Clipper remplace l'historique du presse-papiers de Windows. L'Explorateur redémarre une fois pour libérer la touche : la barre des tâches disparaît deux secondes."
      visual={
        <div className="flex items-center gap-2">
          <Kbd className="h-9 min-w-12 text-sm">Win</Kbd>
          <span className="text-subtle-foreground">+</span>
          <Kbd className="h-9 min-w-9 text-sm">V</Kbd>
        </div>
      }
      footer={
        <>
          <Button variant="ghost" onClick={() => setStep(1)}>
            Garder {settings.shortcut || "mon raccourci"}
          </Button>
          <Button variant="primary" size="lg" disabled={busy} onClick={enableWinV}>
            {busy ? <Spinner className="border-brand-foreground/30 border-t-brand-foreground" /> : null} Utiliser Win+V
          </Button>
        </>
      }
      error={winVError}
    />,
    <Step
      key="paste"
      title="Choisir, c'est coller"
      text="Dans le collage rapide, Entrée colle directement l'élément dans l'application où vous étiez. Maj+Entrée le colle sans mise en forme."
      visual={
        <label className="flex items-center gap-3 text-sm">
          <Switch label="Coller directement" checked={settings.paste_directly} onChange={(v) => update({ paste_directly: v })} />
          Coller directement
        </label>
      }
      footer={
        <Button variant="primary" size="lg" onClick={() => setStep(2)}>
          Continuer <ArrowRight />
        </Button>
      }
    />,
    <Step
      key="startup"
      title="Toujours prêt"
      text="Clipper tourne discrètement dans la zone de notification et ne consomme rien tant que vous ne copiez pas. Tout reste sur cet ordinateur."
      visual={
        <label className="flex items-center gap-3 text-sm">
          <Switch
            label="Démarrer avec Windows"
            checked={settings.launch_at_startup}
            onChange={(v) => update({ launch_at_startup: v })}
          />
          Démarrer avec Windows
        </label>
      }
      footer={
        <Button variant="primary" size="lg" onClick={finish}>
          <Check /> Terminer
        </Button>
      }
    />,
  ];

  return (
    <div className="fixed inset-x-0 top-11 bottom-0 z-30 flex animate-in flex-col items-center justify-center bg-background px-8" role="dialog" aria-label="Bienvenue dans Clipper">
      <Logo className="mb-8 size-12 text-foreground" />
      <div className="w-full max-w-md">{steps[step]}</div>
      <div className="mt-10 flex gap-1.5" aria-hidden="true">
        {steps.map((_, i) => (
          <span key={i} className={cn("h-1 rounded-full transition-all duration-300", i === step ? "w-6 bg-foreground" : "w-1.5 bg-secondary")} />
        ))}
      </div>
    </div>
  );
}

function Step({
  title,
  text,
  visual,
  footer,
  error,
}: {
  title: string;
  text: string;
  visual: React.ReactNode;
  footer: React.ReactNode;
  error?: string | null;
}) {
  return (
    <div className="animate-pop text-center">
      <h1 className="text-2xl text-foreground">{title}</h1>
      <p className="mt-3 text-sm leading-relaxed text-muted-foreground">{text}</p>
      <div className="my-8 flex justify-center">{visual}</div>
      {error && <p className="mb-4 text-13 text-danger">{error}</p>}
      <div className="flex items-center justify-center gap-2">{footer}</div>
    </div>
  );
}
