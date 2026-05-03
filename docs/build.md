# Building claude-kit for local testing

This guide walks through producing a real installable build of claude-kit
for the first time. Day-to-day development still uses `npm run tauri dev`;
this doc only matters when you want a binary you can install and launch
without the dev tooling running.

## TL;DR

```bash
# one-time
npm run tauri icon                    # generate icon variants from src-tauri/icons/icon.png
# update src-tauri/tauri.conf.json → bundle.icon (see step 1)

# every build
npm run tauri build                   # 5–15 min the first time, 1–2 min after
```

Artefacts land in `src-tauri/target/release/bundle/`.

## 1. Prepare the icons (one-time)

`src-tauri/tauri.conf.json` ships with `bundle.icon: []`. The bundler needs
real icon files, so:

```bash
npm run tauri icon
```

This reads `src-tauri/icons/icon.png` (must be 1024×1024, RGBA) and emits
all the variants the bundler needs: `32x32.png`, `128x128.png`,
`128x128@2x.png`, `icon.icns` (macOS), `icon.ico` (Windows), plus a stack
of `Square*Logo.png` files for the Microsoft Store.

Then declare the variants in `tauri.conf.json` so the bundler picks them
up:

```json
"bundle": {
  "active": true,
  "targets": "all",
  "icon": [
    "icons/32x32.png",
    "icons/128x128.png",
    "icons/128x128@2x.png",
    "icons/icon.icns",
    "icons/icon.ico"
  ]
}
```

## 2. Pick where to compile

**Tauri does not cross-compile.** The OS you build on dictates what you
get out:

| Build host                            | Bundle output                          |
| ------------------------------------- | -------------------------------------- |
| WSL / devcontainer / native Linux     | `.deb`, `.rpm`, `.AppImage`            |
| **Windows host** (PowerShell, not WSL)| `.msi` + `.exe` (NSIS)                 |
| macOS                                 | `.dmg` + `.app`                        |

For testing claude-kit as a real Windows user would, build from a Windows
PowerShell session — **not** from inside WSL.

## 3. Windows build (recommended path)

### Prerequisites on the Windows host

1. **Rust** — install via [rustup-init.exe](https://rustup.rs/), accept
   the default `stable-x86_64-pc-windows-msvc` toolchain.
2. **MSVC build tools** — required for the linker. If Visual Studio is
   not installed, get
   [Build Tools for Visual Studio](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
   and select the **"Desktop development with C++"** workload.
3. **Node.js** — install if not already present on the Windows side
   (separate from any Node you may have inside WSL).
4. **WebView2** — already bundled with Windows 11. On Windows 10, install
   the
   [evergreen runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/).

### Build

In **PowerShell** (not WSL):

```powershell
cd C:\path\to\claude-kit
npm install                  # may differ from the WSL-side install
npm run tauri build
```

The first run is slow because Rust does full LTO on every dependency in
release mode. Subsequent builds reuse the cache.

### What you get

```
src-tauri/target/release/
├── claude-kit.exe                                  ← raw binary, runnable as-is
└── bundle/
    ├── msi/   claude-kit_0.1.0_x64_en-US.msi       ← classic MSI installer
    └── nsis/  claude-kit_0.1.0_x64-setup.exe       ← NSIS .exe installer
```

Pick whichever installer flavour you prefer, or just launch
`claude-kit.exe` directly.

## 4. Linux build (from WSL / devcontainer)

If you only need a Linux build (e.g. to sanity-check the WSL/devcontainer
pipeline, or to run on a Linux box):

```bash
npm run tauri build
```

Outputs in `src-tauri/target/release/bundle/`:

- `deb/claude-kit_0.1.0_amd64.deb` — Debian/Ubuntu install
- `rpm/claude-kit-0.1.0-1.x86_64.rpm` — Fedora/RHEL install
- `appimage/claude-kit_0.1.0_amd64.AppImage` — portable, no install needed

The AppImage is the most convenient for quick testing: `chmod +x` then
double-click.

## 5. Things to know before sharing the build

- **No code signing.** Windows SmartScreen will warn the first time
  ("Windows protected your PC") — click *More info → Run anyway*. To
  distribute to other users without warnings, you'd need a code-signing
  certificate (~$200/year for a standard one).
- **No auto-update.** Each new version is a fresh manual install.
- **No system-tray icon.** The app is a regular foreground window.
- **Identifier `dev.claudekit.app`** — fine for personal builds. Change
  it before publishing publicly (e.g. `com.<you>.claudekit`).
- **Library location.** The compiled app uses the same
  `~/.claude-assets/` directory as `npm run tauri dev`. Bundles, library
  edits, and origin metadata are shared between dev and prod runs unless
  you set `CLAUDE_KIT_HOME` to isolate them.

## 6. Troubleshooting

- **`failed to bundle project: ... icon ... not found`** — you forgot
  step 1. Run `npm run tauri icon` and update `bundle.icon` in
  `tauri.conf.json`.
- **`linker 'link.exe' not found`** on Windows — MSVC build tools are
  missing or not in PATH. Reinstall via Build Tools for Visual Studio
  with the C++ workload.
- **"WebView2 not installed"** at launch on Windows 10 — install the
  evergreen runtime (link in step 3).
- **Massive Rust compile times** are normal for the first release build.
  Watch `cargo` output — if it's progressing through crates, just wait.
  If it sits idle for more than a minute, something is stuck (usually a
  network issue fetching crates).
