// Usage: npm run release -- 2.0.1
// Bumps the version in a release/vX.Y.Z branch and opens a pull request that
// merges itself once CI passes. The new version on main then triggers
// .github/workflows/release.yml, which builds and publishes the installers.
// Requires the GitHub CLI (gh) to be logged in.
import { execSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";

const version = process.argv[2]?.replace(/^v/, "");
if (!version || !/^\d+\.\d+\.\d+(-[\w.]+)?$/.test(version)) {
  console.error("Usage : npm run release -- <version>   (ex. 2.0.1)");
  process.exit(1);
}

const run = (cmd) => execSync(cmd, { stdio: "inherit" });
const out = (cmd) => execSync(cmd, { encoding: "utf8" }).trim();

if (out("git status --porcelain")) {
  console.error("Le dépôt contient des modifications non commitées. Commitez-les d'abord.");
  process.exit(1);
}
run("git switch main");
run("git pull --ff-only");

const current = JSON.parse(readFileSync("package.json", "utf8")).version;
if (current === version) {
  console.error(`La version ${version} est déjà celle de main.`);
  process.exit(1);
}

const edit = (file, pattern, replacement) => {
  const text = readFileSync(file, "utf8");
  if (!pattern.test(text)) throw new Error(`Version introuvable dans ${file}`);
  writeFileSync(file, text.replace(pattern, replacement));
};

run(`git switch -c release/v${version}`);
edit("package.json", /"version": "[^"]+"/, `"version": "${version}"`);
edit("package-lock.json", /("name": "clipper",\s+"version": )"[^"]+"/g, `$1"${version}"`);
edit("src-tauri/Cargo.toml", /^version = "[^"]+"/m, `version = "${version}"`);
edit("src-tauri/Cargo.lock", /(name = "clipper"\nversion = )"[^"]+"/, `$1"${version}"`);

run(`git commit -am "Version ${version}"`);
run(`git push -u origin release/v${version}`);
run(`gh pr create --base main --title "Version ${version}" --body "Publication de Clipper ${version}. Voir CHANGELOG.md."`);
run("gh pr merge --auto --squash");
run("git switch main");
console.log(`\nLa PR de la version ${version} sera fusionnée dès que la CI sera verte, puis la release sera publiée.`);
console.log("Suivi : https://github.com/titilyonnais/Clipper/actions");
