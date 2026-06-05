English · [한국어](README.ko.md)

# ✦ Codepop

> Select code anywhere on macOS, press `⌘⇧E`, and get an instant AI explanation in a popup.

Works in any app where you can select text — browser, VS Code, terminal, Slack, PDF.

![Codepop](docs/screenshot.png)

## How it works

1. Select code in any app
2. Press `⌘⇧E`
3. A popup appears next to your cursor → summary + key points + concerns + per-line notes
4. Press `ESC` to close

## Install

1. Download **`Codepop_x.x.x_aarch64.dmg`** from [**Releases**](https://github.com/oxm55522/codepop/releases/latest) → open it → drag `Codepop.app` into **Applications**
2. First launch:
   - Double-click **Codepop** from Launchpad / Applications
   - If you see "unidentified developer / app is damaged":
     - **System Settings → Privacy & Security** → scroll down → click **"Open Anyway"**
     - or in Terminal: `xattr -dr com.apple.quarantine /Applications/Codepop.app`
   - (This is because the app isn't notarized by Apple yet. A signed build removes this step.)
3. A **✦ icon** appears in the menu bar.

> Apple Silicon (M1+) build only.

## First-time setup

On first launch the **Settings window** opens automatically.

1. **Get a free Gemini API key** — [Google AI Studio](https://aistudio.google.com/apikey) → *Create API key*
   - Gemini 2.5 Flash Lite free tier: 15/min, 1500/day → plenty for personal use, **$0**
2. Paste the key into Settings → **Save**
3. Select code and press `⌘⇧E` → **first time it asks for "Accessibility" permission** → allow it
   - If it doesn't prompt: **System Settings → Privacy & Security → Accessibility** → enable Codepop
   - (Needed to read the code you selected in other apps. It does not capture your keystrokes.)

## Settings (menu bar ✦ → Settings)

| Item | Description |
|------|------|
| API key | Your Gemini key |
| Model | `2.5 Flash Lite` (default) / `2.5 Flash` / `2.5 Pro` |
| Shortcut | e.g. `cmd+shift+e` — modifiers (cmd/shift/alt/ctrl) + key (a–z, 0–9) |
| Language | English / 한국어 (also togglable instantly in the popup footer) |

> The UI auto-starts in English on non-Korean systems, and in Korean on Korean systems.

## Features

- **Per-line notes** — the 📝 Notes button shows a one-line note beside each line
- **Line-ref jump** — click `L23` in the explanation to scroll/highlight that line
- **SHA256 caching** — re-selecting the same code shows instantly, no API call
- **EN ↔ KO toggle** — switch instantly from the footer (persisted)
- **Move the window** — drag the popup's top bar
- **Re-analyze** — the 🔄 button (ignores cache)

## Cost / Privacy

- Free model → **$0 API cost**
- The API key is stored locally at `~/.codepop/config.json` (`chmod 600`)
- Cache lives only in `~/.codepop/cache/`, never sent anywhere
- Selected code is sent to Gemini only at analysis time (responses are cached, so re-selecting makes no call)
- Clear sensitive cache: `rm -rf ~/.codepop/cache/*`

## Uninstall

Delete Codepop from Applications + (optionally) `rm -rf ~/.codepop`

## Build (developers)

```bash
npm install
npm run tauri dev      # run in dev
npm run tauri build    # produce .app + .dmg (src-tauri/target/release/bundle/)
```

See [`MIGRATION_TAURI.md`](MIGRATION_TAURI.md) for the migration history and architecture.

## License

MIT — see `LICENSE`.
