import { convertFileSrc } from "@tauri-apps/api/core";

export function cn(...classes: (string | false | null | undefined)[]) {
  return classes.filter(Boolean).join(" ");
}

const rtf = new Intl.RelativeTimeFormat("fr", { numeric: "auto", style: "short" });
const shortDate = new Intl.DateTimeFormat("fr-FR", { day: "numeric", month: "short" });
const shortDateYear = new Intl.DateTimeFormat("fr-FR", { day: "numeric", month: "short", year: "numeric" });

/** "à l'instant", "il y a 5 min", "il y a 3 h", "hier", "il y a 4 j", then the date ("20 sept."). */
export function timeAgo(iso: string): string {
  const then = new Date(iso);
  if (Number.isNaN(then.getTime())) return "";
  const now = new Date();
  const seconds = (now.getTime() - then.getTime()) / 1000;
  if (seconds < 60) return "à l'instant";
  if (seconds < 3600) return rtf.format(-Math.floor(seconds / 60), "minute");
  // Calendar days, so that "hier" is yesterday and not 24 hours ago.
  const midnight = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const days = Math.round((midnight(now) - midnight(then)) / 86_400_000);
  if (days < 1) return rtf.format(-Math.floor(seconds / 3600), "hour");
  if (days < 7) return rtf.format(-days, "day");
  return (then.getFullYear() === now.getFullYear() ? shortDate : shortDateYear).format(then);
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
    powershell: "PowerShell",
    pwsh: "PowerShell",
    cmd: "Invite de commandes",
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
