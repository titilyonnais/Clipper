import { convertFileSrc } from "@tauri-apps/api/core";

export function cn(...classes: (string | false | null | undefined)[]) {
  return classes.filter(Boolean).join(" ");
}

const rtf = new Intl.RelativeTimeFormat("fr", { numeric: "auto", style: "short" });
const STEPS: [Intl.RelativeTimeFormatUnit, number][] = [
  ["year", 31_536_000],
  ["month", 2_592_000],
  ["week", 604_800],
  ["day", 86_400],
  ["hour", 3_600],
  ["minute", 60],
];

/** "il y a 5 min", "hier"… */
export function timeAgo(iso: string): string {
  const seconds = (Date.parse(iso) - Date.now()) / 1000;
  if (!Number.isFinite(seconds)) return "";
  for (const [unit, size] of STEPS) {
    if (Math.abs(seconds) >= size) return rtf.format(Math.round(seconds / size), unit);
  }
  return "à l'instant";
}

const dateFmt = new Intl.DateTimeFormat("fr-FR", { dateStyle: "long", timeStyle: "short" });
export function fullDate(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? "" : dateFmt.format(d);
}

const bytesFmt = new Intl.NumberFormat("fr", { maximumFractionDigits: 1 });
export function humanBytes(n: number): string {
  const units = ["o", "Ko", "Mo", "Go"];
  let i = 0;
  while (n >= 1024 && i < units.length - 1) {
    n /= 1024;
    i++;
  }
  return `${bytesFmt.format(n)} ${units[i]}`;
}

export function plural(n: number, one: string, many = `${one}s`) {
  return `${n.toLocaleString("fr-FR")} ${n > 1 ? many : one}`;
}

export function isEditable(el: Element | null): boolean {
  if (!el) return false;
  const tag = el.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || (el as HTMLElement).isContentEditable;
}

/** Asset URL of an application's cached icon (may not exist). */
export function appIconUrl(dataDir: string, exe: string): string {
  const safe = exe.replace(/[^\p{L}\p{N}._-]/gu, "");
  return convertFileSrc(`${dataDir}\\icons\\${safe}.png`);
}

/** "chrome.exe" -> "Chrome" */
export function appLabel(exe: string): string {
  const base = exe.replace(/\.exe$/i, "");
  const known: Record<string, string> = {
    chrome: "Chrome",
    msedge: "Edge",
    firefox: "Firefox",
    brave: "Brave",
    code: "VS Code",
    explorer: "Explorateur",
    winword: "Word",
    excel: "Excel",
    powerpnt: "PowerPoint",
    outlook: "Outlook",
    olk: "Outlook",
    notepad: "Bloc-notes",
    windowsterminal: "Terminal",
    discord: "Discord",
    slack: "Slack",
    teams: "Teams",
    "ms-teams": "Teams",
    figma: "Figma",
    notion: "Notion",
    claude: "Claude",
    snippingtool: "Capture d'écran",
    screenclippinghost: "Capture d'écran",
  };
  return known[base.toLowerCase()] ?? base.charAt(0).toUpperCase() + base.slice(1);
}
