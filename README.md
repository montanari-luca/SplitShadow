<div align="center">

<img src="assets/brand/splitshadow-logo.png" alt="SplitShadow" width="128" height="128" />

# SplitShadow

### Your windows, one shortcut away.

A tiny, lightning-fast window manager for **macOS** and **Windows**.<br/>
Press a shortcut, type what you want, and your windows snap into place.

> **NOTE**: [Conbind](https://github.com/montanari-luca/Conbind?tab=readme-ov-file) is out, and it has these, and thousands of additional capabilities. This product, is for those who just want a light window manager, Conbind is for those who want the whole power of Spotlight/Raycast in one app. 

[**Download**](https://github.com/montanari-luca/MintFlow-Releases/releases) &nbsp;·&nbsp; [How it works](#how-it-works) &nbsp;·&nbsp; [What you can type](#what-you-can-type) &nbsp;·&nbsp; [FAQ](#faq)

![macOS](https://img.shields.io/badge/macOS-12%2B-black?logo=apple&logoColor=white)
![Windows](https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4?logo=windows&logoColor=white)
![Size](https://img.shields.io/badge/app%20size-~7%20MB-34c759)
![Made by](https://img.shields.io/badge/by-MintFlow%20Technologies-00c896)

</div>

---

## Why you will like it

You already know the pain: a dozen windows, a screen that is never quite big enough, and a mouse you would rather not touch. SplitShadow gives you a Spotlight-style bar that does one thing really well: **arrange your windows**.

- ⚡ **Instant.** Opens the moment you press the shortcut, from any app.
- 🗣️ **Speaks your language.** Type "move window to the left of the screen" or just "left". Typos are fine.
- 🧊 **Looks the part.** A frosted-glass bar that blends into your desktop, in light and dark mode.
- 🪶 **Featherweight.** A few megabytes, no account, no clutter. It lives quietly in your menu bar or tray.

## How it works

1. **Press your shortcut** from anywhere (`Ctrl + Space` on Mac, `Ctrl + Alt + Space` on Windows, and you can change it).
2. **Type what you want.** Suggestions appear as you type, each with a tiny preview of where the window will go.
3. **Press Enter.** Done. The palette disappears and your window is exactly where you asked.

```
┌───────────────────────────────────────────────┐
│ 🔍  left                              Notes   │
├───────────────────────────────────────────────┤
│ ▌▏  Left half                            ⌘1   │
│ ▌▏  Left third                           ⌘2   │
│ ▌▏  Left two thirds                      ⌘3   │
├───────────────────────────────────────────────┤
│ ↑↓ move    ⇥ complete    ↵ run    esc close   │
└───────────────────────────────────────────────┘
```

Click the menu bar or tray icon instead if you prefer. It opens the same palette.

## What you can type

Say it however feels natural. All of these work:

| You type | What happens |
| --- | --- |
| `left`, `right`, `top`, `bottom` | Snap to that half of the screen |
| `move window to the left of the screen` | Same thing, in plain English |
| `upper right corner`, `bottom left` | Snap to a quarter |
| `left third`, `middle third`, `2/3 right` | Thirds and two thirds |
| `maximize`, `almost maximize` | Fill the screen, or 90% of it, centered |
| `center` | Center the window, keep its size |
| `bigger`, `smaller` | Grow or shrink by 10% |
| `next monitor` | Send the window to your other display |
| `hide` | Tuck the window away |
| `undo` | Put the window back where it was |

## Split screen that stays together

Type `split with Slack` and the two windows tile side by side. From then on they are **linked**: drag the edge between them and the other window follows, so there are never gaps or overlaps.

Want to move on? Drag a window away, close it, or type `unlink split`, and they go back to being independent.

## Workspaces

Love how your screen looks right now? Save it.

| You type | What happens |
| --- | --- |
| `save workspace as Dev` | Remember up to four windows and where they are |
| `open Dev` | Bring it all back. Apps that are closed get launched for you |
| `update workspace` | Overwrite the workspace you are in with how things look now |
| `update workspace Writing` | Overwrite a specific one |
| `delete workspace Dev` | Remove it (asks first) |

You can also rename and delete workspaces from Settings.

## Get it

**[Download the latest version](https://github.com/montanari-luca/MintFlow-Releases/releases)** for your system, open it, and you are set. SplitShadow keeps itself up to date: when a new version is ready, it shows up right in the palette.

On **macOS**, the first launch asks for **Accessibility** access. That is how macOS lets an app move other apps' windows, and the welcome screen walks you through it.

## FAQ

**Is it private?**
Yes. SplitShadow has no account, no analytics and no tracking. The only network request it makes is checking for new versions.

**Why does it need Accessibility access on Mac?**
It is the permission macOS requires for any app that moves or resizes other apps' windows. SplitShadow only uses it for that.

**Can I change the shortcut?**
Of course. Open Settings from the tray or menu bar icon, click the shortcut, and press the one you want.

**Does it work with several monitors?**
Yes. The palette appears on the screen your mouse is on, and `next monitor` sends a window across.

**It did not move a window on another desktop (Mac).**
macOS only lets apps see windows on the current Space, so SplitShadow works with what is on your screen right now.

**A window did not quite fill the space.**
A few apps have a minimum size. SplitShadow respects it instead of fighting the app.

**My computer warns me about the app on first launch.**
These early versions are not yet notarized (Mac) or code signed (Windows), so the system may ask you to confirm you want to open it.

## Who made this

<div align="center">

<img src="public/mintflow-logo.webp" alt="MintFlow Technologies" height="44" />

**Developed by MintFlow Technologies**<br/>
Lead developer: **Luca Montanari**

</div>

---

# Contributing

Everything below is for people who want to build or improve SplitShadow.

## Development

Requirements: Node, pnpm, Rust, and the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```bash
pnpm install
pnpm tauri dev        # run the app
pnpm test             # TypeScript tests (matcher, hotkeys, release script)
pnpm test:rust        # Rust tests (layouts, engine, links, workspaces)
```

The built-in stack is Tauri v2, React, Tailwind v4 and Rust. Please keep the build warning-free: `cargo check --all-targets`, `cargo clippy --all-targets`, `pnpm exec tsc --noEmit` and `pnpm build` should all print nothing.

### Project layout

```
src/matcher/       fuzzy search, natural-language intent, workspace and split arguments (pure TS)
src/palette/       the palette UI
src/settings/      settings window: hotkey, workspaces, permissions, updates, about
src-tauri/src/wm/  window management: layout math, engine, linked split, macOS and Windows backends
src-tauri/src/     tray, hotkey, palette window, workspaces store, updater, commands
scripts/           release manifest generator
```

All window logic sits behind the `WindowBackend` trait, so layouts, linking and workspaces are tested against an in-memory fake. The macOS backend (Accessibility API) and the Windows backend (Win32 and WinEvent hooks) need a real desktop to verify, so please test those by hand.

## Releasing

Updates use Tauri's signed updater. The app checks the manifest at
`https://raw.githubusercontent.com/montanari-luca/MintFlow-Releases/refs/heads/main/splitshadow.json`
at launch and every 6 hours.

**Keys.** The updater keypair was generated once with `pnpm tauri signer generate`. The public key is in `src-tauri/tauri.conf.json`. The private key and its password live outside the repo (`~/.tauri/` on the machine that generated them). Back them up: without them you cannot ship updates to existing installs.

**Build.**

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

**Publish.** Generate the manifest, upload everything in `./release` to the GitHub release, then commit `release/splitshadow.json` as `splitshadow.json` in the MintFlow-Releases repo:

```bash
pnpm release:json --base-url https://github.com/montanari-luca/MintFlow-Releases/releases/download/v0.1.0 --notes "What changed"
```

The manifest must be complete: the updater needs a valid `url` and `signature` for the platform it runs on, even when there is no newer version. A placeholder with empty strings makes the check report an error in Settings, Updates.

**Signing and notarization.** An Apple Development identity runs builds on your own Mac. Distributing to other people needs a Developer ID Application certificate and notarization (`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`); Tauri notarizes automatically when they are set. Windows code signing is not set up yet.

## License

Copyright MintFlow Technologies. All rights reserved.
