"use strict";

const { invoke, convertFileSrc } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open: openDialog } = window.__TAURI__.dialog;
const { openUrl } = window.__TAURI__.opener;
const appWindow = window.__TAURI__.window.getCurrentWindow();

const els = {
  bar: document.getElementById("bar"),
  back: document.getElementById("back-btn"),
  forward: document.getElementById("fwd-btn"),
  docName: document.getElementById("doc-name"),
  tabs: document.getElementById("tabs"),
  picker: document.getElementById("theme-picker"),
  openBtn: document.getElementById("open-btn"),
  openMore: document.getElementById("open-more"),
  openMenu: document.getElementById("open-menu"),
  settingsBtn: document.getElementById("settings-btn"),
  settings: document.getElementById("settings-dialog"),
  modeRadios: document.querySelectorAll('#settings-dialog input[name="open-mode"]'),
  emptyOpenBtn: document.getElementById("empty-open-btn"),
  content: document.getElementById("content"),
  empty: document.getElementById("empty"),
  error: document.getElementById("error"),
  errorDetail: document.getElementById("error-detail"),
  themeStyle: document.getElementById("theme"),
};

const state = {
  theme: null,
  themes: [],
  openMode: "tab",
};

const MD_LINK = /\.(md|markdown|mdown|mkd|mdtext|mdtxt|mdwn|mkdn)$/i;
const HAS_SCHEME = /^[a-z][a-z0-9+.-]*:/i;

/* ---------------- paths ---------------- */

/** Resolve `rel` against `dir`, collapsing `.` and `..`. Forward-slash output. */
function resolvePath(dir, rel) {
  let decoded = rel;
  try {
    decoded = decodeURIComponent(rel);
  } catch {
    /* malformed escapes: use the raw text */
  }
  const parts = `${dir}/${decoded}`.replace(/\\/g, "/").split("/");
  const out = [];
  for (let i = 0; i < parts.length; i++) {
    const p = parts[i];
    if (p === "" || p === ".") {
      if (i === 0) out.push(p); // keep a leading empty for UNC-ish roots
      continue;
    }
    if (p === "..") {
      if (out.length > 1) out.pop();
      continue;
    }
    out.push(p);
  }
  return out.join("/");
}

function isRelative(href) {
  return href && !HAS_SCHEME.test(href) && !href.startsWith("//") && !href.startsWith("#");
}

function baseName(p) {
  return p.replace(/[\\/]+$/, "").split(/[\\/]/).pop() || p;
}

/** Windows paths differ in slash and case without being different files. */
function samePath(a, b) {
  return a.replace(/\\/g, "/").toLowerCase() === b.replace(/\\/g, "/").toLowerCase();
}

/* ---------------- tabs ---------------- */

/**
 * A tab owns its own history, so Back in one tab cannot walk into another's
 * documents. `entries` is the visited list, `index` the position in it — the
 * shape is deliberately plain JSON so a tab can be handed to another window
 * through Rust as-is.
 */
let tabs = [];
let activeId = null;
let nextTabId = 1;

/** Closed tabs for Ctrl+Shift+T, most recent last. */
const closedTabs = [];
const CLOSED_LIMIT = 20;

/** Guards against a slow load painting over a newer tab switch. */
let renderToken = 0;

function activeTab() {
  return tabs.find((t) => t.id === activeId) ?? null;
}

function currentEntry(tab) {
  return tab ? (tab.entries[tab.index] ?? null) : null;
}

function makeTab(path) {
  return {
    id: nextTabId++,
    path,
    dir: "",
    label: baseName(path),
    heading: "",
    entries: [{ path, scrollY: 0 }],
    index: 0,
  };
}

/** Snapshot the reading position so Back and tab switches return to the spot. */
function rememberScroll() {
  const entry = currentEntry(activeTab());
  if (entry) entry.scrollY = window.scrollY;
}

/* ---------------- rendering ---------------- */

/** Point relative media at the asset protocol so it loads from disk. */
function resolveMedia(root, dir) {
  root.querySelectorAll("img[src], video[src], audio[src], source[src]").forEach((el) => {
    const raw = el.getAttribute("src");
    if (!isRelative(raw)) return;
    el.setAttribute("src", convertFileSrc(resolvePath(dir, raw)));
  });
}

/** Tables can be arbitrarily wide; give each its own scroll box. */
function wrapTables(root) {
  root.querySelectorAll("table").forEach((table) => {
    if (table.parentElement?.classList.contains("table-scroll")) return;
    const wrap = document.createElement("div");
    wrap.className = "table-scroll";
    table.replaceWith(wrap);
    wrap.appendChild(table);
  });
}

/**
 * Highlight only blocks that declare a language. Auto-detection on unlabeled
 * blocks is frequently wrong and costs real time on large documents.
 */
function highlight(root) {
  root.querySelectorAll("pre > code").forEach((code) => {
    const declared = [...code.classList].some((c) => c.startsWith("language-"));
    if (declared) {
      try {
        hljs.highlightElement(code);
        return;
      } catch {
        /* unknown language: fall through to plain styling */
      }
    }
    code.classList.add("hljs");
  });
}

function show(which) {
  els.content.hidden = which !== "content";
  els.empty.hidden = which !== "empty";
  els.error.hidden = which !== "error";
}

function renderDocument(doc, scrollY) {
  /*
   * Drop any fragment left over from the previous document. Without this, a
   * `#f5` still sitting in the URL means clicking `#f5` in the *next* file is
   * not a change of fragment, so the browser performs no jump at all.
   * replaceState rather than assigning location.hash: no extra entry, no
   * hashchange, no trailing "#".
   */
  if (location.hash) {
    history.replaceState(null, "", location.href.split("#")[0]);
  }

  els.content.innerHTML = doc.html;
  resolveMedia(els.content, doc.dir);
  wrapTables(els.content);
  highlight(els.content);
  show("content");

  // Restore after layout, so the offset being scrolled to actually exists yet.
  requestAnimationFrame(() => window.scrollTo(0, scrollY));
}

/** Render whatever the active tab points at. `scrollY` overrides the saved spot. */
async function showActive(scrollY) {
  const tab = activeTab();
  if (!tab) {
    show("empty");
    updateChrome();
    return;
  }

  const entry = currentEntry(tab);
  const token = ++renderToken;
  try {
    const doc = await invoke("load_file", { path: entry.path });
    if (token !== renderToken) return; // a newer switch already won
    entry.path = doc.path;
    tab.path = doc.path;
    tab.dir = doc.dir;
    tab.label = baseName(doc.path);
    tab.heading = doc.title ?? "";
    renderDocument(doc, scrollY ?? entry.scrollY ?? 0);
  } catch (err) {
    if (token !== renderToken) return;
    els.errorDetail.textContent = String(err);
    show("error");
  }
  updateChrome();
}

function updateChrome() {
  const tab = activeTab();
  els.back.disabled = !tab || tab.index <= 0;
  els.forward.disabled = !tab || tab.index >= tab.entries.length - 1;
  els.docName.textContent = tab ? tab.path : "";
  els.docName.title = tab ? tab.path : "";
  // Only a drag handle while it stands in for a hidden strip.
  els.docName.classList.toggle("handle", tabs.length === 1);
  appWindow
    .setTitle(tab ? `${tab.label} — Markdown Viewer` : "Markdown Viewer")
    .catch(() => {});
  renderTabs();
}

/* ---------------- tab strip ---------------- */

/** Slot the drop caret sits in, or -1 when no drag is hovering this window. */
let dropCaret = -1;

function renderTabs() {
  const visible = tabs.length > 1 || dropCaret >= 0;
  els.tabs.hidden = !visible;
  if (!visible) {
    els.tabs.replaceChildren();
    return;
  }

  const nodes = [];
  tabs.forEach((t, i) => {
    if (i === dropCaret) nodes.push(caretElement());
    const el = document.createElement("div");
    el.className = "tab" + (t.id === activeId ? " active" : "");
    el.dataset.id = String(t.id);
    el.setAttribute("role", "tab");
    el.title = t.heading && t.heading !== t.label ? `${t.heading}\n${t.path}` : t.path;

    const label = document.createElement("span");
    label.className = "tab-label";
    label.textContent = t.label;

    const close = document.createElement("button");
    close.type = "button";
    close.className = "tab-close";
    close.setAttribute("aria-label", `Close ${t.label}`);
    close.textContent = "×";

    el.append(label, close);
    nodes.push(el);
  });
  if (dropCaret >= tabs.length) nodes.push(caretElement());

  els.tabs.replaceChildren(...nodes);
  paintDrag();
}

function caretElement() {
  const c = document.createElement("div");
  c.className = "tab-caret";
  return c;
}

function setCaret(index) {
  if (index === dropCaret) return;
  dropCaret = index;
  renderTabs();
}

/* ---------------- tab operations ---------------- */

let watching = null;

/** Keep the Rust watcher pointed at exactly the files this window has open. */
function syncWatch() {
  const paths = [...new Set(tabs.map((t) => currentEntry(t)?.path).filter(Boolean))];
  const key = paths.join("\0");
  if (key === watching) return;
  watching = key;
  invoke("watch_files", { paths }).catch(console.error);
}

async function openTab(path) {
  const tab = makeTab(path);
  tabs.push(tab);
  rememberScroll();
  activeId = tab.id;
  syncWatch();
  await showActive(0);
}

async function activateTab(id) {
  if (id === activeId) return;
  rememberScroll();
  activeId = id;
  await showActive();
}

/** Drop a tab without recording it as closed — used when it moves elsewhere. */
async function removeTab(id) {
  const i = tabs.findIndex((t) => t.id === id);
  if (i < 0) return null;
  if (id === activeId) rememberScroll();
  const [gone] = tabs.splice(i, 1);
  if (activeId === id) activeId = (tabs[i] ?? tabs[i - 1])?.id ?? null;
  syncWatch();
  await showActive();
  return gone;
}

/**
 * Closing the last tab leaves an empty window rather than destroying it —
 * a window vanishing under you is a worse surprise than an empty one.
 */
async function closeTab(id) {
  const gone = await removeTab(id);
  if (!gone) return;
  closedTabs.push(gone);
  if (closedTabs.length > CLOSED_LIMIT) closedTabs.shift();
}

async function reopenClosed() {
  const tab = closedTabs.pop();
  if (!tab) return;
  rememberScroll();
  tab.id = nextTabId++;
  tabs.push(tab);
  activeId = tab.id;
  syncWatch();
  await showActive();
}

async function cycleTab(step) {
  if (tabs.length < 2) return;
  const i = tabs.findIndex((t) => t.id === activeId);
  const next = tabs[(i + step + tabs.length) % tabs.length];
  await activateTab(next.id);
}

/** Rebuild a tab handed over from another window and take it on. */
async function adoptTab(data, at) {
  const entries =
    Array.isArray(data?.entries) && data.entries.length
      ? data.entries
      : [{ path: data?.path, scrollY: 0 }];
  const index = Math.max(0, Math.min(entries.length - 1, data?.index ?? 0));
  const path = entries[index]?.path;
  if (!path) return;

  rememberScroll();
  const tab = { id: nextTabId++, path, dir: "", label: baseName(path), heading: "", entries, index };
  tabs.splice(Math.max(0, Math.min(tabs.length, at)), 0, tab);
  activeId = tab.id;
  syncWatch();
  await showActive();
  appWindow.setFocus().catch(() => {});
}

/* ---------------- navigation ---------------- */

/** Follow a link in the active tab, discarding any forward history. */
async function loadPath(path) {
  const tab = activeTab();
  if (!tab) return openTab(path);

  const entry = currentEntry(tab);
  if (!entry || !samePath(entry.path, path)) {
    rememberScroll();
    tab.entries.length = tab.index + 1; // drop the forward branch
    tab.entries.push({ path, scrollY: 0 });
    tab.index = tab.entries.length - 1;
    syncWatch();
  }
  await showActive(0);
}

async function go(delta) {
  const tab = activeTab();
  if (!tab) return;
  const target = tab.index + delta;
  if (target < 0 || target >= tab.entries.length) return;

  const from = currentEntry(tab);
  rememberScroll();
  tab.index = target;
  const to = currentEntry(tab);

  /*
   * Anchors put several entries on one document. Stepping between them is a
   * scroll, not a load — re-rendering would flash the page, re-run highlighting
   * and lose nothing but time. The watcher is already pointed at this file too.
   */
  if (from && to && samePath(from.path, to.path)) {
    window.scrollTo(0, to.scrollY ?? 0);
    updateChrome();
    return;
  }

  syncWatch();
  await showActive();
}

/**
 * Record an in-page jump as a history entry, so Back returns to where the link
 * was clicked from rather than skipping the whole document.
 *
 * The browser is left to perform the jump itself. That is what makes `:target`
 * match, which is what keeps the heading clear of the sticky bar — doing the
 * scroll by hand would mean reimplementing that offset.
 */
function pushAnchorEntry(raw) {
  let id = raw;
  try {
    id = decodeURIComponent(raw);
  } catch {
    /* malformed escapes: use the raw text */
  }
  // A link to nothing scrolls nowhere, so it should not cost a Back press.
  if (!document.getElementById(id)) return;

  const tab = activeTab();
  const entry = currentEntry(tab);
  if (!entry) return;

  rememberScroll(); // the spot being left, captured before the browser moves
  tab.entries.length = tab.index + 1; // drop the forward branch
  tab.entries.push({ path: entry.path, scrollY: 0, hash: id });
  tab.index = tab.entries.length - 1;
  updateChrome();

  // The jump happens after this handler returns; record where it landed so
  // Forward comes back to exactly the same place.
  requestAnimationFrame(() => {
    const landed = currentEntry(activeTab());
    if (landed?.hash === id) landed.scrollY = window.scrollY;
  });
}

/** Re-render the open document in place: no history entry, no scroll jump. */
async function refresh() {
  if (!activeTab()) return;
  await showActive(window.scrollY);
}

/** Where a newly opened file goes, per the Settings choice. */
async function openDocument(path) {
  if (state.openMode === "window" && tabs.length > 0) {
    await invoke("open_window", { path });
  } else {
    await openTab(path);
  }
}

/* ---------------- open mode ---------------- */

/**
 * Reflect the mode in the UI without writing it back. Also called when another
 * window changes it, so it must not re-broadcast.
 */
function showOpenMode(mode) {
  state.openMode = mode;
  for (const radio of els.modeRadios) radio.checked = radio.value === mode;
  els.openBtn.title =
    mode === "window"
      ? "Open a Markdown file in a new window (Ctrl+O)"
      : "Open a Markdown file in a new tab (Ctrl+O)";
}

function setOpenMode(mode) {
  showOpenMode(mode);
  invoke("set_open_mode", { mode }).catch(console.error);
}

/** `undefined` toggles. */
function showOpenMenu(open) {
  const next = open ?? els.openMenu.hidden;
  els.openMenu.hidden = !next;
  els.openMore.setAttribute("aria-expanded", String(next));
}

/* ---------------- dragging ---------------- */

/*
 * HTML5 drag-and-drop cannot cross a webview boundary, and each window here is
 * its own webview. So dragging is done with pointer capture: the button stays
 * down, this window keeps receiving moves even outside its own bounds, and Rust
 * answers "which of my windows is under this screen point".
 */

const DRAG_THRESHOLD = 5; // px before a click becomes a drag
const PROBE_MS = 30; // throttle for the cross-window hit test
const STRIP_SLACK = 24; // vertical grace before a drag counts as leaving

let drag = null;
let ghostEl = null;

/**
 * Pointer position as a physical screen point, from the window's own origin and
 * scale. Deliberately not `screenX * devicePixelRatio`: that assumes one scale
 * factor for the whole desktop and lands in the wrong place as soon as two
 * monitors are set to different scaling.
 */
function screenPoint(event, origin) {
  return {
    x: origin.x + event.clientX * origin.scale,
    y: origin.y + event.clientY * origin.scale,
  };
}

/** A chip that follows the cursor once the tab leaves the strip. */
function moveGhost(event) {
  if (!ghostEl) {
    ghostEl = document.createElement("div");
    ghostEl.className = "tab-ghost";
    document.body.appendChild(ghostEl);
  }
  const tab = tabs.find((t) => t.id === drag.id);
  ghostEl.textContent = tab ? tab.label : "";
  ghostEl.style.transform = `translate(${event.clientX - 16}px, ${event.clientY - 14}px)`;
  ghostEl.hidden = false;
}

function hideGhost() {
  if (ghostEl) ghostEl.hidden = true;
}

/** Reflect drag state in the DOM. Safe to call with no drag in progress. */
function paintDrag() {
  const detached = Boolean(drag?.detached);
  els.tabs.querySelectorAll(".tab").forEach((el) => {
    const mine = Number(el.dataset.id) === drag?.id;
    el.classList.toggle("dragging", Boolean(drag?.moved) && mine && !detached);
    el.classList.toggle("detached", detached && mine);
  });
  document.documentElement.classList.toggle("dragging-tab", Boolean(drag?.moved));
  if (!detached) hideGhost();
}

/** Slot a tab released at `clientX` would occupy, in current DOM order. */
function insertionIndex(clientX) {
  const nodes = [...els.tabs.querySelectorAll(".tab")];
  for (let i = 0; i < nodes.length; i++) {
    const r = nodes[i].getBoundingClientRect();
    if (clientX < r.left + r.width / 2) return i;
  }
  return nodes.length;
}

function reorderTo(target) {
  const from = tabs.findIndex((t) => t.id === drag.id);
  if (from < 0) return;
  // The slot index counts the dragged tab, which is about to be lifted out.
  let to = target > from ? target - 1 : target;
  to = Math.max(0, Math.min(tabs.length - 1, to));
  if (to === from) return;
  const [moved] = tabs.splice(from, 1);
  tabs.splice(to, 0, moved);
  renderTabs();
}

function beginDrag(event, id) {
  event.preventDefault();
  drag = {
    id,
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY,
    moved: false,
    detached: false,
    lastProbe: 0,
    origin: null,
  };
  // Fetched rather than derived in JS so the screen mapping is exact; it lands
  // before the pointer has moved far enough to count as a drag.
  invoke("window_origin")
    .then((o) => {
      if (drag) drag.origin = o;
    })
    .catch(console.error);
  // Capture on the bar, not the tab or the strip: reordering rebuilds the tab
  // elements mid-drag, and the path handle lives outside the strip entirely.
  els.bar.setPointerCapture(event.pointerId);
}

function onTabPointerDown(event) {
  const el = event.target.closest(".tab");
  if (!el) return;
  const id = Number(el.dataset.id);

  if (event.button === 1) {
    event.preventDefault(); // no autoscroll
    closeTab(id);
    return;
  }
  if (event.button !== 0 || event.target.closest(".tab-close")) return;
  beginDrag(event, id);
}

/**
 * With a single document the strip is hidden, so the path doubles as its drag
 * handle. Tearing off is meaningless there — the document already has a window
 * to itself — but dragging it onto another window merges the two.
 */
function onDocNamePointerDown(event) {
  if (event.button !== 0 || !els.tabs.hidden || activeId === null) return;
  beginDrag(event, activeId);
}

function onDragMove(event) {
  if (!drag || event.pointerId !== drag.pointerId) return;

  if (!drag.moved) {
    const dx = event.clientX - drag.startX;
    const dy = event.clientY - drag.startY;
    if (Math.hypot(dx, dy) < DRAG_THRESHOLD) return;
    drag.moved = true;
  }

  // A hidden strip has a zero-sized rect sitting at the origin, which would
  // otherwise read as "inside" for any pointer near the top of the window.
  const strip = els.tabs.getBoundingClientRect();
  const inStrip =
    !els.tabs.hidden &&
    event.clientY >= strip.top - STRIP_SLACK &&
    event.clientY <= strip.bottom + STRIP_SLACK;

  if (inStrip) {
    if (drag.detached) {
      drag.detached = false;
      invoke("drag_cancel").catch(() => {});
    }
    reorderTo(insertionIndex(event.clientX));
    paintDrag();
    return;
  }

  drag.detached = true;
  paintDrag();
  moveGhost(event);

  if (!drag.origin) return; // origin still in flight; nothing to map with yet
  const now = performance.now();
  if (now - drag.lastProbe < PROBE_MS) return;
  drag.lastProbe = now;
  const { x, y } = screenPoint(event, drag.origin);
  invoke("drag_over", { x, y }).catch(console.error);
}

async function onDragEnd(event) {
  if (!drag || event.pointerId !== drag.pointerId) return;
  const d = drag;
  drag = null;
  try {
    els.bar.releasePointerCapture(d.pointerId);
  } catch {
    /* capture already gone */
  }
  hideGhost();
  paintDrag();

  if (!d.moved) {
    await activateTab(d.id);
    return;
  }
  if (!d.detached) {
    renderTabs(); // reorder is already applied; just drop the drag styling
    return;
  }

  const tab = tabs.find((t) => t.id === d.id);
  if (!tab) return;
  const origin = d.origin ?? (await invoke("window_origin").catch(() => null));
  if (!origin) return;
  const { x, y } = screenPoint(event, origin);
  try {
    const outcome = await invoke("drop_tab", {
      x,
      y,
      tab: { path: tab.path, entries: tab.entries, index: tab.index },
    });
    // "adopted" — another window took it. "detached" — it became a new window.
    if (outcome === "adopted" || outcome === "detached") await removeTab(d.id);
  } catch (err) {
    console.error(err);
  }
}

function onDragCancel() {
  if (!drag) return;
  const detached = drag.detached;
  drag = null;
  hideGhost();
  paintDrag();
  if (detached) invoke("drag_cancel").catch(() => {});
  renderTabs();
}

function onTabClick(event) {
  const close = event.target.closest(".tab-close");
  if (!close) return;
  const el = close.closest(".tab");
  if (el) closeTab(Number(el.dataset.id));
}

/* ---------------- themes ---------------- */

async function applyTheme(name) {
  try {
    els.themeStyle.textContent = await invoke("read_theme", { name });
    state.theme = name;
    els.picker.value = name;
  } catch (err) {
    console.error("theme load failed", name, err);
  }
}

async function selectTheme(name) {
  await applyTheme(name);
  await invoke("set_theme", { name });
}

async function loadThemeList() {
  state.themes = await invoke("list_themes");
  els.picker.replaceChildren(
    ...state.themes.map((t) => {
      const opt = document.createElement("option");
      opt.value = t.name;
      opt.textContent = t.label;
      return opt;
    }),
  );
  if (state.theme) els.picker.value = state.theme;
}

function cycleTheme(step) {
  if (!state.themes.length) return;
  const i = state.themes.findIndex((t) => t.name === state.theme);
  const next = state.themes[(i + step + state.themes.length) % state.themes.length];
  selectTheme(next.name);
}

/* ---------------- interactions ---------------- */

async function pickFile() {
  const picked = await openDialog({
    multiple: false,
    filters: [
      { name: "Markdown", extensions: ["md", "markdown", "mdown", "mkd", "mdtext", "mdwn"] },
      { name: "All files", extensions: ["*"] },
    ],
  });
  return typeof picked === "string" ? picked : null;
}

/**
 * The webview is the whole app: letting it navigate away would leave a dead
 * window. Intercept every link — follow anchors, open sibling documents in
 * place, hand everything else to the system browser.
 */
function onLinkClick(event) {
  const a = event.target.closest("a[href]");
  if (!a) return;
  const href = a.getAttribute("href");
  if (!href) return;

  // In-page anchor: the browser performs the jump, we just record it.
  if (href.startsWith("#")) {
    pushAnchorEntry(href.slice(1));
    return;
  }

  event.preventDefault();

  if (isRelative(href)) {
    const [pathPart] = href.split("#");
    const target = resolvePath(activeTab()?.dir ?? "", pathPart);
    if (MD_LINK.test(pathPart)) {
      loadPath(target);
    } else {
      openUrl(convertFileSrc(target)).catch(console.error);
    }
    return;
  }

  openUrl(href).catch(console.error);
}

async function onKeydown(event) {
  if (event.key === "F8") {
    event.preventDefault();
    cycleTheme(event.shiftKey ? -1 : 1);
    return;
  }

  // Otherwise the dialog is modal: let it own the keyboard, Escape included.
  if (els.settings.open) return;

  if (event.key === "Escape" && !els.openMenu.hidden) {
    showOpenMenu(false);
    els.openMore.focus();
    return;
  }

  if (event.altKey && !event.ctrlKey && !event.metaKey) {
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      go(-1);
      return;
    }
    if (event.key === "ArrowRight") {
      event.preventDefault();
      go(1);
      return;
    }
  }

  const ctrl = event.ctrlKey || event.metaKey;
  if (!ctrl) return;

  if (event.key === "Tab") {
    event.preventDefault();
    cycleTab(event.shiftKey ? -1 : 1);
    return;
  }

  const key = event.key.toLowerCase();
  if (key === "o") {
    event.preventDefault();
    const p = await pickFile();
    if (p) await openDocument(p);
  } else if (key === "t" && event.shiftKey) {
    event.preventDefault();
    await reopenClosed();
  } else if (key === "t") {
    event.preventDefault();
    const p = await pickFile();
    if (p) await openTab(p);
  } else if (key === "n") {
    event.preventDefault();
    const p = await pickFile();
    if (p) await invoke("open_window", { path: p });
  } else if (key === "w") {
    event.preventDefault();
    if (activeId !== null) await closeTab(activeId);
  } else if (key === "r") {
    event.preventDefault();
    refresh();
  }
}

/** Thumb buttons on a mouse, as in a browser. */
function onMouseUp(event) {
  if (event.button === 3) {
    event.preventDefault();
    go(-1);
  } else if (event.button === 4) {
    event.preventDefault();
    go(1);
  }
}

/* ---------------- boot ---------------- */

async function main() {
  els.openBtn.addEventListener("click", async () => {
    showOpenMenu(false);
    const p = await pickFile();
    if (p) await openDocument(p);
  });
  els.emptyOpenBtn.addEventListener("click", async () => {
    const p = await pickFile();
    if (p) await openDocument(p);
  });

  els.openMore.addEventListener("click", () => showOpenMenu());
  els.openMenu.addEventListener("click", async (e) => {
    const item = e.target.closest("button[data-mode]");
    if (!item) return;
    showOpenMenu(false);
    const p = await pickFile();
    if (!p) return;
    // Deliberately bypasses openDocument: the point of this menu is to override
    // the saved default for one file without changing it.
    if (item.dataset.mode === "window") await invoke("open_window", { path: p });
    else await openTab(p);
  });
  // Capture, so a click anywhere else dismisses the menu before that click does
  // whatever else it was going to do.
  document.addEventListener(
    "pointerdown",
    (e) => {
      if (!els.openMenu.hidden && !e.target.closest("#open-split")) showOpenMenu(false);
    },
    true,
  );

  els.settingsBtn.addEventListener("click", () => {
    showOpenMenu(false);
    els.settings.showModal();
  });
  for (const radio of els.modeRadios) {
    radio.addEventListener("change", () => {
      if (radio.checked) setOpenMode(radio.value);
    });
  }

  els.picker.addEventListener("change", (e) => selectTheme(e.target.value));
  els.content.addEventListener("click", onLinkClick);
  els.back.addEventListener("click", () => go(-1));
  els.forward.addEventListener("click", () => go(1));

  els.tabs.addEventListener("pointerdown", onTabPointerDown);
  els.docName.addEventListener("pointerdown", onDocNamePointerDown);
  // Capture lands on the bar, so the whole drag is tracked from there.
  els.bar.addEventListener("pointermove", onDragMove);
  els.bar.addEventListener("pointerup", onDragEnd);
  els.bar.addEventListener("pointercancel", onDragCancel);
  els.tabs.addEventListener("click", onTabClick);

  document.addEventListener("keydown", onKeydown);
  document.addEventListener("mouseup", onMouseUp);
  // Chromium fires auxclick for the thumb buttons too; swallow it so the
  // default "navigate" behaviour cannot fight our own handling.
  document.addEventListener("auxclick", (e) => {
    if (e.button === 3 || e.button === 4) e.preventDefault();
  });

  const settings = await invoke("get_settings");
  showOpenMode(settings.open_mode ?? "tab");
  await loadThemeList();
  await applyTheme(settings.theme);

  /*
   * Addressed to this window only. `listen()` defaults to the `Any` target,
   * which also receives events Rust sent to a *different* window — that would
   * open every warm file in every window, and let one dropped tab be adopted by
   * all of them at once. `AnyLabel` is the same variant `emit_to(label)`
   * produces, so the two match exactly.
   */
  const listenHere = (event, handler) =>
    listen(event, handler, { target: { kind: "AnyLabel", label: appWindow.label } });

  await listenHere("file-opened", (e) => openTab(e.payload));
  await listenHere("file-changed", (e) => {
    const changed = e.payload ?? [];
    const open = currentEntry(activeTab())?.path;
    if (open && changed.some((c) => samePath(c, open))) refresh();
  });
  await listen("themes-changed", async () => {
    await loadThemeList();
    if (state.theme) await applyTheme(state.theme);
  });
  await listen("open-mode-changed", (e) => showOpenMode(e.payload));

  // Another window's tab is hovering over this one.
  await listenHere("tab-drag-over", (e) => {
    setCaret(tabs.length > 1 ? insertionIndex(e.payload.x) : tabs.length);
  });
  await listenHere("tab-drag-out", () => setCaret(-1));
  await listenHere("tab-adopt", async (e) => {
    const at = dropCaret >= 0 ? dropCaret : tabs.length;
    setCaret(-1);
    await adoptTab(e.payload.tab, at);
  });

  // Whatever this window was created to show: a file-association open, or a
  // tab torn off another window.
  const pending = await invoke("take_pending");
  if (pending?.kind === "path") {
    await openTab(pending.path);
  } else if (pending?.kind === "tab") {
    await adoptTab(pending.tab, 0);
  } else {
    show("empty");
    updateChrome();
  }

  // Window starts hidden so the first frame is already themed and painted.
  await appWindow.show();
}

main().catch((err) => {
  console.error(err);
  els.errorDetail.textContent = String(err);
  show("error");
  appWindow.show();
});
