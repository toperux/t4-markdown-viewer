---
name: drive-app
description: Launch the T4 Markdown Viewer debug build and drive it for real — run JS in the page, click elements, take screenshots, and answer the native Windows Open / Select Folder dialogs. Use whenever a UI change needs checking in the running app, for smoke tests, screenshots, theme checks, or reproducing a frontend bug, even if the user only says "try it" or "does it work". The frontend has no test harness, so this is the only way to see behaviour instead of assuming it.
---

# Driving the app

`cargo test` covers only Rust. The frontend (`src/app.js`, a plain script, no
bundler) is checked by launching the debug exe with WebView2 remote debugging
on and talking Chrome DevTools Protocol to it. Windows only, as written.

Scripts in `.claude/skills/drive-app/scripts/` (run from the repo root):

- `cdp.mjs` — `node cdp.mjs <port> eval "<js>"` | `click "<css selector>"` |
  `shot <out.png>`. Uses Node 24's global `WebSocket`; nothing to install.
- `dialog.ps1` — answers the native open dialog: `-ProcId <pid> -Path <path>`,
  `-Cancel`, or `-Dump`.

## 1. Before launching

- **Leave the user's viewer open; rename the debug build instead.**
  Single-instance locks on the app identifier (`{identifier}-sim`), so a debug
  exe built from `tauri.conf.json` as-is hands its argv to the installed app
  (`%LOCALAPPDATA%\T4 Markdown Viewer\t4-markdown-viewer.exe`) and exits. Build
  with an override and the two run side by side:

  ```bash
  TAURI_CONFIG='{"identifier":"com.montevirgen.t4-markdown-viewer.audit"}' \
    cargo build --manifest-path src-tauri/Cargo.toml
  ```

  A plain `cargo build` afterwards restores the real identifier; tauri-build
  re-runs when `TAURI_CONFIG` changes. Only a test that needs the real
  identifier justifies closing the user's app, and that's their call, so ask.
  Note its window title first (`Get-Process t4-markdown-viewer | select
  MainWindowTitle`), because a plain relaunch doesn't restore tabs.
- **Settings are shared too.** Theme, open mode and the update toggle live in
  `%APPDATA%\t4-markdown-viewer\config.json`. Back it up to the scratchpad now:
  `selectTheme`, the light/dark button and F8 all write it. The user's viewer
  stays open and may save its own changes meanwhile, so at the end restore it
  only if the debug app changed it — diff the live file against the backup
  first, and copy back only that change.
- **Port.** `netstat -ano | grep 9222`. t4-git-ui often holds 9222; use 9223.
- **Build.** Use the `TAURI_CONFIG` command above, and rebuild after every
  `src/` edit: frontend assets are embedded at build time. Themes are copied into
  `target/debug/themes/` and the exe prefers that copy, so after editing
  `src-tauri/themes/*.css` rebuild or copy them across. A plain `cargo build`
  restores the real identifier, so don't run one until you're done.

## 2. Launch

In the background, since it doesn't exit:

```bash
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223 \
  "<repo>/src-tauri/target/debug/t4-markdown-viewer.exe" [file.md]
```

Then, in a separate call (a `taskkill … && launch` chain aborts when nothing was
running): wait ~5 s, get the PID with `Get-Process t4-markdown-viewer | select
Id,Path` (`dialog.ps1` needs it), and take a first screenshot. Read the PNG. A
blank frame means the page never loaded.

## 3. Drive

`app.js` isn't a module, so its top-level names are page globals:

- state: `tabs`, `activeId`, `activeTab()`, `state` (`.theme`, `.themes`,
  `.folder`), `els` (cached elements), `pickerOpen`
- actions: `openTab(path)`, `openDocument(path)`, `closeTab(id)`,
  `openFolder(path)`, `closeFolder()`, `showFolder()`, `applyTheme(name)` (not
  saved), `selectTheme(name)` (saved), `toggleThemeMode()`

Names drift; grep `src/app.js` before relying on one.

Use `click` rather than `el.click()` inside `eval` when the handler needs a real
user gesture: clipboard writes, and anything that opens a native dialog.
`Input.dispatchMouseEvent` counts as one; a scripted `.click()` may not.

Check visibility through computed style, not `hidden` alone. Much of the UI is
shown by CSS selectors (e.g. `#sidebar[data-tree="empty"] + main .hint-empty`):

```js
const v = (s) => { const e = document.querySelector(s);
  return !!e && getComputedStyle(e).display !== "none" && !e.closest("[hidden]"); };
```

Read every screenshot you take. For theme work, also measure: loop
`applyTheme` over `state.themes` and read computed colours. That leaves the
user's setting alone, and numbers like contrast ratios catch what a glance at
fifteen screenshots misses.

Pass paths into `eval` with forward slashes (`F:/x/y`). Backslashes go through
bash quoting and then JS string escaping, and one layer too few turns `\n` into
a newline.

## 4. Native dialogs

`#empty-open-btn`, `#empty-folder-btn`, `#sidebar-name`, the bar's Open button
and Ctrl+O open the real Windows dialog, which CDP can't see. `dialog.ps1` finds
it (a `#32770` window owned by the app's PID), sets its name box with
`WM_SETTEXT` and sends the OK or Cancel command. It uses Win32 because UI
Automation doesn't expose that box or those buttons.

```bash
node .claude/skills/drive-app/scripts/cdp.mjs 9223 click "#empty-open-btn"
powershell -NoProfile -ExecutionPolicy Bypass -File .claude/skills/drive-app/scripts/dialog.ps1 -ProcId <pid> -Path 'F:\path\to\file.md'
```

- The file dialog closes after one call.
- The **folder picker needs two**: the first navigates into the typed folder,
  the second (`-Path ''`) selects it.
- `-Cancel` presses Cancel. `-Dump` lists the dialog's edit boxes and buttons,
  for when Windows changes the layout.
- The script's last line says whether the dialog is still open. Confirm the
  outcome with `eval` (`tabs`, `state.folder`, `pickerOpen` back to `false`).

## 5. Clean up

1. `taskkill //PID <pid>`. The double slash matters: MSYS mangles `/PID` into a
   path. A debug app once ignored the plain kill (three later tries didn't
   reproduce it); `//F` works.
2. Only after the process is gone, since it may write on exit: diff the live
   `config.json` against the backup, and if the debug app changed it, copy
   back only that change, not the whole backup. Re-read the file to confirm.
3. If you closed the user's viewer, reopen it detached:
   `powershell -NoProfile -Command "Start-Process '<path>\t4-markdown-viewer.exe'"`.
   A background bash job would die with the session. To bring back the file it
   had open, add `-ArgumentList '\"<file path>\"'`.

## Other gotchas

- **Spaces in paths.** The repo sits under `_ pet projects`. Wrap repeated
  script calls in a shell function that passes `"$@"`, not in a command stored
  in a variable: an unquoted `$D -Path …` split the script path at the space,
  and every dialog call failed.

- **Window size.** `core:window:allow-set-size` isn't granted, so resize with
  Win32 `MoveWindow` from PowerShell.
- **Clipboard.** Seed it with `powershell Set-Clipboard -Value x` and read it
  back with `Get-Clipboard -Raw`. The copy-section button uses a promise-valued
  `ClipboardItem`; that's been checked on WebView2 only.
- **Cross-window tab drag** can be driven from one window's CDP session:
  `Input.dispatchMouseEvent` accepts coordinates outside the viewport, and
  pointer capture on `#bar` keeps delivering them. Map the target window's
  screen position into the source's client CSS px with `invoke("window_origin")`
  on both. The windows mustn't overlap, because Rust uses `WindowFromPoint`.
- **Closing a window from JS** needs `core:window:allow-close` in
  `capabilities/default.json`. `core:default` doesn't include it, and callers
  that catch and log fail silently.
- **Several windows.** `cdp.mjs` drives the first tauri page in `/json/list`.
  Edit its `find` to target another.
