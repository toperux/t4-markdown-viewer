# XWayland Freeze Plan

> **Status (2026-10-20):** executed on the owner's go (65a0a47, plus a comment fixup from the
> change review); walked on the Linux VM (steps 1-4); change review clean; triaged below. Step 5,
> the shipped AppImage, waits for the release.

**Spec:** the open item "On Linux X11, a long or image-heavy document can freeze the page"
(`docs/open-items.md` → Deferred), which the owner set for its own plan right after the restore,
tab drag and Linux text fixes shipped (v1.8.7). Its trigger asked for the AppImage to be probed
first; the probe ran on 2026-10-19.

**Architecture:** one environment variable set at startup in `src-tauri/src/main.rs`, beside
`GDK_BACKEND`, with a unit test; plus the records. No frontend change, no new dependency.

## What the user sees

The AppImage on a Wayland desktop shows its first paint and then nothing: no scrolling, no
pictures, no live reload, though the window still resizes. Every launch.

## Measured (Linux VM, GNOME 50 on Wayland, Ubuntu 26.04, a virtual GPU)

- **The shipped v1.8.7 AppImage** (sha256 checked; bundled WebKitGTK 2.50.4, loaded from its
  mount), no driver attached, real keys through Mutter's RemoteDesktop D-Bus API: frozen in 26 of
  26 launches (`kitchen-sink.md` 10/10, `slow.md` 10/10, a session restore 3/3 each). Frozen =
  no pixel changed after End, a rewrite of the open file, and Home.
- **With `WEBKIT_DISABLE_DMABUF_RENDERER=1`:** 0 of 26; every trigger repainted.
- A frozen web process sleeps in `poll_schedule_timeout` at ~7-8% CPU while idle (0% when working).
- Earlier, through WebDriver: the debug build under `GDK_BACKEND=x11` (system WebKitGTK 2.52.6)
  froze too, on v1.8.6 and later, and a native Wayland launch never did (2026-10-08: 0 of 3).
- **The variable's cost** (`slow.md`, 20 real PageDowns over ~2.1 s, 5 runs each, the same bundled
  WebKit run natively on Wayland as the baseline, since DMA-BUF can't run under XWayland):

  | Mode | Web process CPU while scrolling | Main process | Idle | Memory |
  |---|---|---|---|---|
  | Wayland, default | 45% | 13% | 0% | ~450 MB web |
  | Wayland, variable set | 72% | 18% | 0% | ~441 MB web |
  | XWayland, variable set | 72% | 18% | 0% | ~435 MB web |

  About 1.6× the web process's CPU while scrolling; memory unchanged. Smoothness unmeasured
  (reasoned: the shared-memory copy path, which could drop frames on a slow machine).
- **Not measured:** real hardware (the VM has a virtual GPU, and DMA-BUF is a GPU path: the freeze
  may be this VM's, or every XWayland desktop's); a native Xorg session (it needs a logout); other
  distributions and compositors (KDE).

## Current behaviour (main at e679e69)

- `gdk_backend` (`main.rs:1794-1803`) returns `x11` inside an AppImage (`APPIMAGE` set), else
  none. `main` (`1819-1828`) sets `GDK_BACKEND` from it before any thread starts, then
  `exec_with_unicode_args()`. The programs the app starts inherit it.
- So the AppImage always runs under XWayland on a Wayland desktop, and that is where it freezes.
  A .deb or .rpm runs native Wayland unless the user forces X11 (`GDK_BACKEND=x11`), or runs an
  Xorg session.
- Unit test `gdk_backend_is_x11_only_inside_an_appimage` (`2058-2063`).

## The change

Beside `GDK_BACKEND`, inside the same Linux block of `main` (before `exec_with_unicode_args()`
and before `tauri::Builder`, so still before any thread), set `WEBKIT_DISABLE_DMABUF_RENDERER=1`
when the app will run X11 on a Wayland desktop (Q1, option b), unless the user set the variable
(their value wins, either way). The decision is a pure function of the raw values, next to
`gdk_backend`, so the parsing is unit-tested too:

```rust
/// Whether to turn off WebKitGTK's DMA-BUF renderer: under XWayland it shows the
/// first paint and then never repaints (measured, WebKitGTK 2.50 and 2.52), and
/// the AppImage runs there on a Wayland desktop. Costs ~1.6x the web process's
/// CPU while scrolling (measured). A value the user set is left alone.
#[cfg(target_os = "linux")]
fn disable_dmabuf(
    gdk_backend: Option<&str>,
    wayland_display: bool,
    session_type: Option<&str>,
    user_set: bool,
) -> bool {
    // GTK tries a comma list in order; the first entry is what opens.
    let x11 = gdk_backend.and_then(|b| b.split(',').next()).map(str::trim) == Some("x11");
    let wayland = wayland_display || session_type == Some("wayland");
    !user_set && x11 && wayland
}
```

`main` passes `GDK_BACKEND` as it stands after the AppImage line and `XDG_SESSION_TYPE`, each as
`std::env::var(..).ok()` bound first and `.as_deref()` in the call (bound with `.as_deref()`
already, the temporary drops: E0716); whether `WAYLAND_DISPLAY` is set; and whether
`WEBKIT_DISABLE_DMABUF_RENDERER` is present at all (`var_os(..).is_some()`).

- **Detection, case by case** (reasoned from GTK 3.24's backend choice, not read in source here):
  `x11` or `x11,wayland` on a Wayland desktop → X11 through XWayland → set; `wayland,x11`, `*` or
  unset → native Wayland first → not set; empty → not X11 (GTK likely opens nothing) → not set;
  an Xorg session (no `WAYLAND_DISPLAY`, `XDG_SESSION_TYPE=x11`) → not set; a Wayland session
  launched without `WAYLAND_DISPLAY` (`env -i`, some launchers) → `XDG_SESSION_TYPE=wayland`
  still sets it. A runtime check after GTK starts would be exact, but the variable must be set
  before any thread, so the environment is the only race-free place.
- **WebKit's reading** (from memory of WebKit's source, unverified): the UI process reads it once,
  at the first webview, and disables DMA-BUF for any value but `0`; the web process inherits the
  environment. So `1` is right, a user's `0` keeps DMA-BUF on, and the presence test fits. No
  dependency touches the variable (wry 0.57.0, tauri-runtime-wry 2.12.1, tao 0.37.1, tauri 2.12.2,
  webkit2gtk 2.0.2: grepped). The probe exported it from a shell; `set_var` is the same `setenv`
  that `GDK_BACKEND` already relies on.
- **One test**, in the file's style (one sentence-named test, several asserts), e.g.
  `dmabuf_is_off_only_for_x11_on_wayland_unless_the_user_set_it`: `x11`, `x11,wayland`,
  `wayland,x11`, unset, empty, no Wayland, session type only, user set. (The `trim` is defensive
  only: GTK likely rejects ` x11` itself.)

**Q1 (for the owner): where to set it.**

| Option | Covers | Cost lands on |
|---|---|---|
| (a) Inside an AppImage only | the measured case (26/26) | every AppImage user, a native Xorg session included (unmeasured whether it freezes there) |
| (b) Whenever the app runs X11 on a Wayland desktop (XWayland) | the AppImage, and a .deb run with `GDK_BACKEND=x11` (both measured to freeze) | only XWayland runs |
| (c) Whenever the app runs X11 at all | (b) plus a native Xorg session (unmeasured) | Xorg users too, who may not need it |

*Q1, answered by the owner 2026-10-19: (b). It covers exactly what was measured to freeze, and
leaves native Wayland and a native Xorg session as they are; if Xorg freezes too, it stays broken
until a report (a new open item).*

**Also inherited:** like `GDK_BACKEND`, the variable passes to the programs the app starts:
`xdg-open` and its kin through the opener (links, files), so a WebKitGTK program opened from the
app renders without DMA-BUF too; the update's restart and the non-Unicode re-exec
(`exec_with_unicode_args`, `1779-1792`), which then see it as the user's and keep it, value `1`.
`reveal_path` goes over D-Bus and starts nothing. Reasoned harmless beyond the CPU cost.

## Verification

1. `cargo fmt --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings` and
   `cargo test`, as CI runs them. The new test is `#[cfg(target_os = "linux")]`, so it doesn't run
   on the Windows host: run it on the Linux VM, or rely on CI's Linux leg.
2. **Linux VM** (report only), a locally built AppImage of the change, on Wayland, no driver, real
   keys as in the probe: 10 launches each of `kitchen-sink.md` and `slow.md`, a session restore
   3 each: 0 frozen. The *web process's* `/proc/<pid>/environ` shows the variable (the main
   process's environ never shows a `set_var`).
3. The same with `WEBKIT_DISABLE_DMABUF_RENDERER=0` exported by the user: the app leaves it, and
   freezes as before (the user's value wins).
4. The negatives, by the web process's environ:
   - the .deb build natively on Wayland: not set, and scrolling CPU matches the default row above;
   - with `GDK_BACKEND=x11`: set, no freeze; `x11,wayland`: set; `wayland,x11`: not set;
   - the AppImage with `env -u WAYLAND_DISPLAY XDG_SESSION_TYPE=x11` (a stand-in for an Xorg
     session): not set.
5. Once the release is out: the shipped AppImage, 5 launches each document: 0 frozen.

## Records

- Move the open item from `docs/open-items.md` (`## Deferred`) to `docs/closed-items.md`
  (`## Deferred`) with a *Fixed* note: the probe, the cost, and what is still unmeasured (real
  hardware, native Xorg, KDE).
- A new open item under `docs/open-items.md` → `## Deferred`: a native Xorg session is unmeasured
  and left alone; reopen on a report of the page freezing there.
- The README's Linux section (`README.md:173-198`) says nothing of X11 or Wayland: whether the
  variable's cost or the override belongs there goes to the owner as a question at change review.

## Release

A fix to shipped behaviour; worth a patch soon, since the AppImage is unusable on a Wayland desktop
as measured. One commit, no prefix, release-note summary such as "The Linux AppImage no longer
freezes after its first paint on Wayland".

## Walks and triage (2026-10-20)

- Gates on the VM: fmt, clippy clean; `cargo test` 219 passed, the new test included.
- A locally built AppImage of 65a0a47 (host WebKitGTK 2.52.6) on Wayland, no driver: 0 of 26
  frozen (v1.8.7: 26 of 26), the variable `1` in every web process. A user's `0`: kept, 6 of 6
  frozen. Native Wayland and `wayland,x11`: not set; `x11`, `x11,wayland`: set, no freeze.
- The Xorg stand-in (both Wayland markers removed) wasn't set, as designed, and froze: it still
  ran under XWayland, so it says nothing about a real Xorg session.
- Scroll CPU with 2.52.6: ~59-60% without the variable, ~65-68% with it (a few runs).
- Triage, by the owner:
  1. A launch under XWayland without Wayland markers isn't detected: **accepted** with a trigger,
     with native Xorg (`open-items.md`).
  2. One AppImage launch never opened a window, the second Linux launch failure in the day's
     walks: **deferred** as an item; later Linux walks log every launch.
  3. The README: **no change**.
