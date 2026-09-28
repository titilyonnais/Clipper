// Smart previews: what a piece of text is, beyond its type.

const HEX = /^#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const FUNC = /^(?:rgba?|hsla?|oklch|oklab|hwb)\(\s*[\d.%\s,/+-]+\)$/i;

/** CSS colour when the whole text is one (#hex, rgb(), hsl(), oklch()…). */
export function colorOf(text: string): string | null {
  const t = text.trim();
  if (t.length > 60) return null;
  if (HEX.test(t) || FUNC.test(t)) {
    return CSS.supports("color", t) ? t : null;
  }
  // "ff8800" without the hash is too ambiguous (could be an id): ignored.
  return null;
}

export function urlParts(text: string): { host: string; rest: string } | null {
  try {
    const u = new URL(text.trim());
    if (u.protocol === "mailto:") return { host: u.pathname, rest: "" };
    const rest = `${u.pathname === "/" ? "" : u.pathname}${u.search}${u.hash}`;
    return { host: u.host.replace(/^www\./, ""), rest };
  } catch {
    return null;
  }
}

/** Indented JSON when the text is valid, compact JSON. */
export function prettyJson(text: string): string | null {
  const t = text.trim();
  if (!(t.startsWith("{") || t.startsWith("["))) return null;
  try {
    const pretty = JSON.stringify(JSON.parse(t), null, 2);
    return pretty === t ? null : pretty;
  } catch {
    return null;
  }
}

export function lineCount(text: string): number {
  let n = 1;
  for (let i = 0; i < text.length; i++) if (text.charCodeAt(i) === 10) n++;
  return n;
}

export function wordCount(text: string): number {
  return (text.match(/[\p{L}\p{N}]+/gu) ?? []).length;
}
