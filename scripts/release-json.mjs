#!/usr/bin/env node
// Builds the updater manifest (splitshadow.json) from finished Tauri bundles.
//
//   node scripts/release-json.mjs --base-url https://github.com/<org>/<repo>/releases/download/v0.1.0 [--notes "text"]
//
// Looks for these bundle outputs (build them first, with TAURI_SIGNING_PRIVATE_KEY set):
//   macOS    src-tauri/target/universal-apple-darwin/release/bundle/macos/SplitShadow.app.tar.gz (+ .sig)
//            falling back to the host architecture build in src-tauri/target/release/bundle/macos
//   Windows  src-tauri/target/release/bundle/nsis/SplitShadow_<version>_x64-setup.exe (+ .sig)
//
// Copies the artifacts into ./release with versioned names and writes ./release/splitshadow.json.
// Upload everything in ./release to the GitHub release, then commit splitshadow.json to the
// MintFlow-Releases repo.

import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/** Pure: assemble the manifest. `artifacts` maps a platform key to { file, signature }. */
export function buildManifest({ version, notes = "", pubDate, baseUrl, artifacts }) {
  const platforms = {};
  for (const [key, a] of Object.entries(artifacts)) {
    platforms[key] = { signature: a.signature.trim(), url: `${baseUrl.replace(/\/+$/, "")}/${encodeURIComponent(a.file)}` };
  }
  return { version, notes, pub_date: pubDate, platforms };
}

/** Pure: which bundle files to look for, relative to src-tauri. */
export function artifactCandidates(version, arch = process.arch) {
  const hostKey = arch === "arm64" ? "darwin-aarch64" : "darwin-x86_64";
  return {
    mac: [
      { from: "target/universal-apple-darwin/release/bundle/macos/SplitShadow.app.tar.gz", name: `SplitShadow_${version}_universal.app.tar.gz`, keys: ["darwin-aarch64", "darwin-x86_64"] },
      { from: "target/aarch64-apple-darwin/release/bundle/macos/SplitShadow.app.tar.gz", name: `SplitShadow_${version}_aarch64.app.tar.gz`, keys: ["darwin-aarch64"] },
      { from: "target/release/bundle/macos/SplitShadow.app.tar.gz", name: `SplitShadow_${version}_macos.app.tar.gz`, keys: [hostKey] },
    ],
    windows: [
      { from: `target/release/bundle/nsis/SplitShadow_${version}_x64-setup.exe`, name: `SplitShadow_${version}_x64-setup.exe`, keys: ["windows-x86_64"] },
    ],
  };
}

function arg(name) {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 ? process.argv[i + 1] : undefined;
}

function main() {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  const conf = JSON.parse(readFileSync(join(root, "src-tauri/tauri.conf.json"), "utf8"));
  const version = conf.version;
  const baseUrl = arg("base-url");
  if (!baseUrl) {
    console.error("Missing --base-url (the folder URL the files will be uploaded to).");
    process.exit(1);
  }
  const out = join(root, "release");
  mkdirSync(out, { recursive: true });

  const artifacts = {};
  const cands = artifactCandidates(version);
  for (const group of [cands.mac, cands.windows]) {
    const hit = group.find((c) => existsSync(join(root, "src-tauri", c.from)) && existsSync(join(root, "src-tauri", c.from + ".sig")));
    if (!hit) continue;
    const src = join(root, "src-tauri", hit.from);
    copyFileSync(src, join(out, hit.name));
    copyFileSync(src + ".sig", join(out, hit.name + ".sig"));
    const signature = readFileSync(src + ".sig", "utf8");
    for (const key of hit.keys) artifacts[key] = { file: hit.name, signature };
  }
  if (!Object.keys(artifacts).length) {
    console.error("No signed bundles found. Build with TAURI_SIGNING_PRIVATE_KEY set, then run again.");
    process.exit(1);
  }
  const missing = ["darwin-aarch64", "darwin-x86_64", "windows-x86_64"].filter((k) => !artifacts[k]);
  if (missing.length) console.warn(`Warning: no artifact for ${missing.join(", ")}. The updater errors on those platforms until the manifest has them.`);

  const manifest = buildManifest({ version, notes: arg("notes") ?? "", pubDate: new Date().toISOString(), baseUrl, artifacts });
  writeFileSync(join(out, "splitshadow.json"), JSON.stringify(manifest, null, 2) + "\n");
  console.log(`Wrote ${join(out, "splitshadow.json")} for v${version}. Upload the contents of ./release to the GitHub release.`);
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
