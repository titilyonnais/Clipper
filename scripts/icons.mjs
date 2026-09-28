// Generates every icon from the logo geometry: `node scripts/icons.mjs`.
// Then `npx tauri icon src-tauri/icons/app-icon.png` derives the .ico and
// the PNG sizes used by the installer.
import { Resvg } from "@resvg/resvg-js";
import { mkdirSync, writeFileSync } from "node:fs";

// Logo: a rounded square holding three history lines of decreasing length.
// `weight` thickens the strokes for tiny sizes (notification area).
function glyph(color, weight = 1) {
  const frame = 5 * weight;
  const line = 4.5 * weight;
  return `
    <rect x="${10 + frame / 2 - 2.5}" y="${10 + frame / 2 - 2.5}" width="${44 - frame + 5}" height="${44 - frame + 5}"
      rx="11" fill="none" stroke="${color}" stroke-width="${frame}"/>
    <path d="M21 25.5H43M21 32.5H38M21 39.5H31" stroke="${color}" stroke-width="${line}" stroke-linecap="round"/>`;
}

const svg = (body, size = 64) =>
  `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="${size}" height="${size}">${body}</svg>`;

// Crossed-out variant: the glyph is cut along the slash so it stays legible.
const paused = (color, weight) => `
  <defs><mask id="m"><rect width="64" height="64" fill="#fff"/>
    <path d="M8 60L60 8" stroke="#000" stroke-width="${10 * weight}"/></mask></defs>
  <g mask="url(#m)">${glyph(color, weight)}</g>
  <path d="M10 58L58 10" stroke="${color}" stroke-width="${4.5 * weight}" stroke-linecap="round"/>`;

function png(markup, size) {
  return new Resvg(markup, { fitTo: { mode: "width", value: size } }).render().asPng();
}

mkdirSync("src-tauri/icons/tray", { recursive: true });

// Application icon: white glyph on a black tile, legible on light and dark taskbars.
const tile = `<rect width="64" height="64" rx="14" fill="#000"/><g transform="translate(6 6) scale(0.8125)">${glyph("#fff", 1.15)}</g>`;
writeFileSync("src-tauri/icons/app-icon.png", png(svg(tile), 1024));

for (const [name, color] of [
  ["white", "#fff"],
  ["black", "#000"],
]) {
  writeFileSync(`src-tauri/icons/tray/tray-${name}.png`, png(svg(glyph(color, 1.35)), 32));
  writeFileSync(`src-tauri/icons/tray/tray-${name}-paused.png`, png(svg(paused(color, 1.35)), 32));
}

// Logo for the interface and the repository (follows the text colour).
writeFileSync("assets/logo.svg", svg(glyph("currentColor"), 64).replace(' width="64" height="64"', ""));
writeFileSync("public/clipper.svg", svg(tile, 64));
console.log("Icons generated.");
