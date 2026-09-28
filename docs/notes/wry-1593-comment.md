Drafted, not posted: a comment for tauri-apps/wry#1593 (2026-09-28).

Another consequence on Linux, in case it helps prioritise: a navigation handler that keeps the page on the app (return `false` for anything but the app's own URL) silently blocks every **`srcdoc` iframe** there. The handler is called with `about:srcdoc`, wry calls `webkit_policy_decision_ignore`, and the frame stays on its empty initial document. No `load` fires and no CSP violation is reported. On Windows the same handler never sees the frame, so it loads.

Minimal repro, after the page has loaded, with such a handler:
```js
const f = document.createElement("iframe");
f.onload = () => console.log("load");
f.srcdoc = "<p>hi</p>";
document.body.append(f);
```
It logs on Windows and never logs on Linux. The same `decide-policy` ignore in plain WebKitGTK (python3-gi, WebKit2 4.1) reproduces it without wry.

Workaround: allow exactly `about:srcdoc` in the handler. A top-level page can't navigate there, so only a subframe gets through.

wry 0.55.1 (tauri 2.11.6), WebKitGTK 2.52.6, Ubuntu 24.04.
