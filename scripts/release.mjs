// Usage: npm run release -- 2.0.1
// Sets the version everywhere, commits, tags and pushes. GitHub Actions then
// builds the installers and publishes the release (.github/workflows/release.yml).
import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2]?.replace(/^v/, "");
if (!version || !/^\d+\.\d+\.\d+(-[\w.]+)?$/.test(version)) {
  console.error("Usage : npm run release -- <version>   (ex. 2.0.1)");
  process.exit(1);
}

const run = (cmd) => execSync(cmd, { stdio: "inherit" });
const dirty = execSync("git status --porcelain", { encoding: "utf8" }).trim();
if (dirty) {
  console.error("Le dépôt contient des modifications non commitées. Commitez-les d'abord.");
  process.exit(1);
}

const edit = (file, pattern, replacement) => {
  const text = readFileSync(file, "utf8");
  if (!pattern.test(text)) throw new Error(`Version introuvable dans ${file}`);
  writeFileSync(file, text.replace(pattern, replacement));
};

edit("package.json", /"version": "[^"]+"/, `"version": "${version}"`);
edit("package-lock.json", /("name": "clipper",\s+"version": )"[^"]+"/g, `$1"${version}"`);
edit("src-tauri/Cargo.toml", /^version = "[^"]+"/m, `version = "${version}"`);
edit("src-tauri/Cargo.lock", /(name = "clipper"\nversion = )"[^"]+"/, `$1"${version}"`);

run(`git commit -am "Release v${version}"`);
run(`git tag v${version}`);
run("git push --follow-tags");
console.log(`\nv${version} poussée. Suivez la compilation : https://github.com/titilyonnais/Clipper/actions`);
