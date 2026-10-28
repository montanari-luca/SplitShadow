# SplitShadow

A fast, keyboard-driven window manager for macOS and Windows, built with Tauri v2, React and Tailwind.

Press a global shortcut (or click the tray / menu bar icon) and a frosted-glass command palette opens in the middle of your screen. Type what you want, in plain words, and it happens to the window you were just using.

Developed by **MintFlow Technologies**. Lead developer: **Luca Montanari**.

## What it does

- **Snap and resize:** left, right, top, bottom, thirds, two thirds, quarters, maximize, almost maximize (90%), center, make larger or smaller, next or previous display, restore, hide.
- **Natural language:** `left`, `move window to the left of the screen`, `upper right corner`, `2/3 on the right` and `snap lft` all work. Matching is fuzzy, typo tolerant and fully offline.
- **Linked split:** `split with Slack` tiles two windows 50/50. Resize one and the other follows. Dragging a window away, closing it or minimizing it ends the link.
- **Workspaces (1 to 4 windows):** `save workspace as Dev`, `open Dev`, `update workspace` (the active one) or `update workspace Dev`, `delete workspace Dev`. Opening launches any missing apps.
- **Auto updates:** signed updates from a JSON file on GitHub.

Palette keys: Up/Down move, Tab completes, Enter runs, Esc closes, Cmd/Ctrl+1-9 runs row N.

## Using it

1. Launch SplitShadow. On first run, grant **Accessibility** access on macOS (System Settings, Privacy & Security, Accessibility).
2. Press the shortcut (default `Ctrl+Space` on macOS, `Ctrl+Alt+Space` on Windows) from any app. Change it in Settings.
3. Type a command and press Enter.

Known limits:

- macOS only exposes windows on the **current Space** to the Accessibility API, so windows on other desktops cannot be listed or placed.
- Some apps enforce a minimum window size. SplitShadow reads back the real size and accepts the overlap instead of fighting the app.

## Development

Requirements: Node, pnpm, Rust, and the Tauri v2 prerequisites for your OS.

```bash
pnpm install
pnpm tauri dev        # run the app
pnpm test             # TypeScript tests (matcher, hotkeys, release script)
pnpm test:rust        # Rust tests (layouts, engine, links, workspaces)
```

Layout:

```
src/matcher/       fuzzy search, natural-language intent, workspace/split arguments (pure TS)
src/palette/       the palette UI
src/settings/      settings window: hotkey, workspaces, permissions, updates, about
src-tauri/src/wm/  window management: layout math, engine, linked split, macOS and Windows backends
src-tauri/src/     tray, hotkey, palette window, workspaces store, updater, commands
scripts/           release manifest generator
```

All window logic sits behind the `WindowBackend` trait, so layouts, linking and workspaces are tested against an in-memory fake. The macOS backend (Accessibility API) and Windows backend (Win32 and WinEvent hooks) need a real desktop to verify.

## Releasing

Updates use Tauri's signed updater. The app checks the manifest at
`https://raw.githubusercontent.com/montanari-luca/MintFlow-Releases/refs/heads/main/splitshadow.json`
at launch and every 6 hours.

### Keys

The updater keypair was generated once with `pnpm tauri signer generate`. The **public** key is in `src-tauri/tauri.conf.json`. The **private** key and password live outside the repo (`~/.tauri/splitshadow.key` and `~/.tauri/splitshadow.key.password` on the machine that generated them). Back them up: without them you cannot ship updates to existing installs.

### Build

```bash
export TAURI_SIGNING_PRIVATE_KEY="$HOME/.tauri/splitshadow.key"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$(cat ~/.tauri/splitshadow.key.password)"

# macOS (universal), signed with your Apple identity
export APPLE_SIGNING_IDENTITY="<name of your signing identity>"
rustup target add x86_64-apple-darwin aarch64-apple-darwin
pnpm tauri build --target universal-apple-darwin

# Windows (on a Windows machine)
pnpm tauri build
```

Then generate the manifest and upload:

```bash
pnpm release:json --base-url https://github.com/montanari-luca/MintFlow-Releases/releases/download/v0.1.0 --notes "What changed"
```

Upload everything in `./release` to the GitHub release and commit `release/splitshadow.json` as `splitshadow.json` in the MintFlow-Releases repo.

The manifest **must be complete**: the updater needs a valid `url` and `signature` for the platform it runs on, even when there is no newer version. A placeholder with empty strings makes the check report an error (shown in Settings, Updates).

### Signing and notarization

- **macOS:** an *Apple Development* identity is enough to run builds on your own Mac and keeps the Accessibility permission stable across rebuilds. To distribute to other people you need a **Developer ID Application** certificate and notarization (`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`); Tauri notarizes automatically when they are set.
- **Windows:** code signing is not set up yet, so SmartScreen will warn on first run.

## License

Copyright MintFlow Technologies. All rights reserved.
