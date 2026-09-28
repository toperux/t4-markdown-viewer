# Linux walk gotchas — plan

**Goal.** Put what the 2026-09-28 Linux run (Ubuntu 26.04 VM) learned into
`.claude/skills/drive-app/linux.md`, so the next run doesn't rediscover it; and
make its clean-up kill by recorded process ID, per the owner's rule, instead of
by `pkill` pattern.

**Source.** The VM session's two reports of that run: the checks (recorded in
`docs/open-items.md`, the Mermaid item) and how it recorded and killed its
process IDs. Only what it measured goes in as fact; what it reasoned (the dconf
links, the port owners being its own, plain `&` for tauri-driver) is marked as
such, and the last is proven by the Verify run.

**Scope.** One file, `.claude/skills/drive-app/linux.md`. No app code.

Already in `linux.md`, so not added: the XDG settings isolation (§1 *Give it
its own settings*), and running on the live Wayland desktop (*On the live
Wayland desktop*).

## Outcome

Done 2026-09-28. All six edits landed as planned. The Verify run on the Ubuntu
26.04 VM passed:

- tauri-driver outlived its Bash call under a plain `&`.
- `$S/pids` held Xvfb, tauri-driver and WebKitWebDriver, matching `ss -ltnp`
  and `pgrep -af`.
- After the kill, all three checks printed nothing.

It also settled a parentage question. `ps -o pid,ppid` showed Xvfb and
tauri-driver reparented away from the Bash call, WebKitWebDriver as
tauri-driver's child, and the app as WebKitWebDriver's child.

The run waited `sleep 1` between the kill and the checks. That wait is now in
§5 step 2, marked as untried without it. The change review also found one
paragraph badly wrapped by edit 5, and it was rewrapped.

Still unmeasured, and marked as such in `linux.md`:

- the OS switch under Xvfb
- GTK without the dconf links
- `$!` for tauri-driver without `setsid`
- the checks without the wait

## Edits

### 1. §3 Drive — a real Ctrl+wheel

In the xdotool code block (`linux.md:117-124`), after the `mousemove … click 1`
line, add:

```bash
xdotool mousemove 500 300 keydown ctrl click 4 keyup ctrl  # Ctrl+wheel up, one notch
```

After the paragraph that follows the block (`linux.md:126-129`), add:

> Buttons 4 and 5 are the wheel, up and down, at the pointer; `keydown` /
> `keyup` round them make a chord. One notch reached the page as a `wheel` with
> `deltaY` -90 and `ctrlKey` true. With the window at 0,0 (no window manager),
> page coordinates are screen coordinates: a `mousemove 500 300` read back in
> the page as 500,300.

### 2. New section, after *On the live Wayland desktop* — the OS light/dark switch

> ## The OS light/dark switch
>
> A check that the page follows the OS ran on the live desktop (above);
> whether a switch reaches the app under Xvfb wasn't tried. Switching changes
> the look of the user's desktop: ask first, and note their values to put
> back.
>
> ```bash
> gsettings get org.gnome.desktop.interface color-scheme
> gsettings get org.gnome.desktop.interface gtk-theme
> ```
>
> - **Link the desktop's settings into the scratch config.** With
>   `XDG_CONFIG_HOME` in the scratchpad, GTK looks for the desktop's settings
>   there. Before launching, link in the three folders the 2026-09-28 run
>   linked (with `S` set, as in §2):
>   `for d in dconf gtk-3.0 gtk-4.0; do ln -s ~/.config/$d $S/config/$d; done`.
>   That GTK misses the switch without them is that run's reasoning; it
>   wasn't tried without.
> - **GNOME has two switches, and the page is dark if either is.**
>   `color-scheme` (Settings' Dark Style: `prefer-dark` / `default`) and
>   `gtk-theme` (a dark one such as Ubuntu's default `Yaru-dark`, or
>   `Adwaita-dark`). Light needs both light. So set one light and flip the
>   other: under `Yaru-dark`, flipping `color-scheme` alone never reaches the
>   page — GNOME's doing, not a failure.
> - **Check the page saw it** before judging anything that follows the OS: an
>   `eval` that adds a `change` listener to
>   `matchMedia('(prefers-color-scheme: dark)')` and counts, then `matches`
>   after each switch. No change event means the webview never saw it.
> - **Put the user's values back** with `gsettings set` when done.
>
> Measured 2026-09-28 on Ubuntu 26.04.1, GNOME 50.1, Wayland, WebKitGTK
> 2.52.6, with the Yaru and Adwaita pairs.

### 3. §2 Launch — start the helpers with `&` and record their IDs

Why: §5 stops the helpers with `pkill -f` patterns, which also kill another
driving session's helpers on the same machine (another repo's walk). The
owner's rule is to kill by recorded process ID only. What the VM run measured,
2026-09-28:

- A plain `&` inside a normal Bash call: Xvfb outlived the call and was killed
  from a later one. tauri-driver was started under `setsid` (run 1) and `nohup`
  (run 2), and outlived its call both times; under a plain `&` it is untested,
  which the Verify run below proves or disproves.
- `$!` after `Xvfb … &` was Xvfb's ID; killing it worked.
- `$!` after `setsid tauri-driver … &` named a process already gone; without
  `setsid` it wasn't captured, so it is untested.
- `ss -ltnp` named the right tauri-driver (port 4444) and WebKitWebDriver (port
  4445) in both runs. That they were this run's own is reasoned: the ports were
  free just before launch.

Replace `linux.md:72-83` (from `Two background jobs, then the session.` through
the end of the code block) with:

> Two helpers, then the session. Start the helpers with a plain `&` inside a
> Bash call, not as the tool's background jobs: they outlive the call, and `&`
> gives Xvfb's process ID. Env and shell variables do not carry between Bash
> calls, so each command carries what it needs — set `S` again in every call
> that uses it:
>
> ```bash
> S=<scratchpad>/app
> Xvfb :99 -screen 0 1600x1000x24 >/dev/null 2>&1 &
> echo $! > $S/pids                                  # Xvfb's ID
> DISPLAY=:99 GDK_BACKEND=x11 TAURI_WEBVIEW_AUTOMATION=true \
>   XDG_CONFIG_HOME=$S/config XDG_DATA_HOME=$S/data XDG_CACHE_HOME=$S/cache \
>   tauri-driver > $S/tauri-driver.log 2>&1 &
> ```
>
> ```bash
> node .claude/skills/drive-app/scripts/wd.mjs start \
>   "$PWD/src-tauri/target/debug/t4-markdown-viewer" "$PWD/examples/kitchen-sink.md"
> ```
>
> **Record the drivers' IDs** once `start` returns, so clean-up kills only what
> this run started. Take them from the ports, not from `$!`: after `setsid
> tauri-driver &`, `$!` named a process already gone (without `setsid`,
> untested), and WebKitWebDriver is started by tauri-driver, not by you. §1
> checked both ports were free, so their owners now are this run's:
>
> ```bash
> S=<scratchpad>/app
> ss -ltnp | grep -E ':444[45] ' | grep -o 'pid=[0-9]*' | cut -d= -f2 >> $S/pids
> ```

(The paragraph after the block, `Paths to the app … never loaded.`, stays.)

### 4. §5 Clean up, step 2 — kill by recorded ID

Replace step 2 (`linux.md:159-162`) with:

> 2. `kill $(sort -u $S/pids); rm -f $S/pids` (with `S` set; `sort -u`
>    because a process listening on two addresses is listed twice). The `rm`
>    matters: §2 appends, and an ID left from an earlier run may belong to
>    someone else's process by now. Then check nothing is left: `ss -ltn | grep -E
>    ':444[45] '` and `ls /tmp/.X11-unix/ | grep X99` print nothing, and so does
>    `pgrep -af '^[^ ]*(target/debug/t4-markdown-viewer|tauri-driver|WebKitWebDriver|Xvfb :99)'`.
>    That `pgrep` only looks: whatever it lists is another session's or a leak.
>    Report it; don't kill it.

### 5. *On the live Wayland desktop* — the same recording

After `run the same tauri-driver command without Xvfb, DISPLAY=:99 or
GDK_BACKEND, and ask the user first: the window appears on their desktop.`
(`linux.md:167-169`), add:

> Record the drivers' IDs from their ports as in §2. With no Xvfb nothing
> truncates `$S/pids`, so check it isn't there first. If it is, an earlier run
> never finished clean-up: don't kill its IDs, which may be reused by now —
> delete the file and look for leftovers with §5's `pgrep`.

### 6. §5 Clean up, step 3

Replace `3. Nothing to restore: the user's settings were never touched.` with:

> 3. Nothing to restore: the user's settings were never touched — unless the
>    OS light/dark switch was flipped; then put their `gsettings` values back
>    (see *The OS light/dark switch*).

## Verify

- `linux.md` reads straight through; the new section's references (*On the
  live Wayland desktop*, `$S`, §2, §5) resolve.
- Every added claim traces to the VM report, or is marked as reasoning.
- The VM session runs §1's port check, §2's launch and ID recording, and §5
  steps 1–2 exactly as written, headless, once. Pass: `$S/pids` holds three
  IDs after `sort -u` — Xvfb, tauri-driver, WebKitWebDriver, matching `ss
  -ltnp` and `pgrep -af` — and after clean-up the three checks print nothing.
  It reports without editing; a failure comes back here as a finding.

## Commit

`docs: add Ctrl+wheel and the OS light/dark switch to drive-app on Linux, and stop its helpers by process ID`
— local, on main.
