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
  folderBtn: document.getElementById("folder-btn"),
  sidebar: document.getElementById("sidebar"),
  sidebarName: document.getElementById("sidebar-name"),
  sidebarClose: document.getElementById("sidebar-close"),
  tree: document.getElementById("tree"),
  settingsBtn: document.getElementById("settings-btn"),
  settings: document.getElementById("settings-dialog"),
  modeRadios: document.querySelectorAll('#settings-dialog input[name="open-mode"]'),
  autoUpdate: document.getElementById("auto-update"),
  checkNow: document.getElementById("check-now"),
  updateStatus: document.getElementById("update-status"),
  appVersion: document.getElementById("app-version"),
  updateBtn: document.getElementById("update-btn"),
  updateDialog: document.getElementById("update-dialog"),
  updateSummary: document.getElementById("update-summary"),
  updateNotes: document.getElementById("update-notes"),
  updateWarning: document.getElementById("update-warning"),
  updateProgress: document.getElementById("update-progress"),
  updateError: document.getElementById("update-error"),
  updateNotesBtn: document.getElementById("update-notes-btn"),
  updateNow: document.getElementById("update-now"),
  emptyOpenBtn: document.getElementById("empty-open-btn"),
  content: document.getElementById("content"),
  empty: document.getElementById("empty"),
  image: document.getElementById("image"),
  imageView: document.getElementById("image-view"),
  imageEl: document.getElementById("image-el"),
  imageTools: document.getElementById("image-tools"),
  zoomLevel: document.getElementById("zoom-level"),
  error: document.getElementById("error"),
  errorDetail: document.getElementById("error-detail"),
  themeStyle: document.getElementById("theme"),
};

const state = {
  theme: null,
  themes: [],
  openMode: "tab",
  /**
   * Platform facts, filled in from `get_settings` at boot. The defaults are the
   * conservative reading — no cross-window drag, case-sensitive paths — so the
   * app behaves correctly for the brief moment before the answer arrives.
   */
  crossWindowDrag: false,
  caseInsensitivePaths: false,
  /** The release `check_for_update` found, or null while there is none. */
  update: null,
  /** Root of the folder in the sidebar, or null while it is closed. */
  folder: null,
};

const MD_LINK = /\.(md|markdown|mdown|mkd|mdtext|mdtxt|mdwn|mkdn)$/i;
const IMG_LINK = /\.(svg|png|jpe?g|gif|webp|avif|bmp|ico)$/i;
const HAS_SCHEME = /^[a-z][a-z0-9+.-]*:/i;

/**
 * What a tab holds is read back off its path rather than stored beside it. That
 * keeps `makeTab`, `adoptTab` and the cross-window drag payload untouched: a
 * tab handed to another window arrives knowing what it is.
 */
function isImage(p) {
  return IMG_LINK.test(p);
}

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

/** Parent of `p`, or "" when it has none. A drive root keeps its slash: `C:\` not `C:`. */
function dirName(p) {
  const m = p.match(/^(.*)[\\/][^\\/]+$/);
  if (!m) return "";
  if (/^[a-z]:$/i.test(m[1])) return `${m[1]}\\`;
  return m[1] || "/";
}

/**
 * Windows paths differ in slash without being different files, and on Windows
 * and macOS in case too. On Linux `Notes.md` and `notes.md` are two documents,
 * so folding case there would quietly merge them into one tab.
 */
function samePath(a, b) {
  return normPath(a) === normPath(b);
}

/** Slash- and, where the platform folds it, case-normalised: the form paths compare in. */
function normPath(p) {
  const slashed = p.replace(/\\/g, "/");
  return state.caseInsensitivePaths ? slashed.toLowerCase() : slashed;
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
    /** Pictures the rendered document shows, so the watcher covers them too. */
    media: [],
    entries: [{ path, scrollY: 0 }],
    index: 0,
  };
}

/** Snapshot the reading position so Back and tab switches return to the spot. */
function rememberScroll() {
  const entry = currentEntry(activeTab());
  if (!entry) return;
  // A picture scrolls inside its own box, and in two directions; it also has a
  // zoom to keep. `picture` is null until one is actually on screen.
  if (picture && isImage(entry.path)) {
    rememberImage();
    return;
  }
  entry.scrollY = window.scrollY;
}

/* ---------------- rendering ---------------- */

/**
 * How many times each file has been asked for afresh. The asset protocol sends no
 * caching headers and the app never navigates away, so the webview hands back the
 * copy it already has for a URL it has already fetched; a version in the query
 * string is what makes new bytes a new URL. A file is bumped when it is seen to
 * change, and again when it is shown while unwatched — nobody was listening, so
 * the cached copy proves nothing.
 */
const assetVersions = new Map();

/** Unversioned until the file's first bump, so untouched pictures stay cached. */
function assetUrl(file) {
  const v = assetVersions.get(normPath(file));
  return v ? `${convertFileSrc(file)}?v=${v}` : convertFileSrc(file);
}

function bumpAsset(file) {
  const key = normPath(file);
  assetVersions.set(key, (assetVersions.get(key) ?? 0) + 1);
}

/** Point relative media at the asset protocol so it loads from disk. */
function resolveMedia(root, dir) {
  root.querySelectorAll("img[src], video[src], audio[src], source[src]").forEach((el) => {
    const raw = el.getAttribute("src");
    if (!isRelative(raw)) return;
    const file = resolvePath(dir, raw);
    // A picture nothing was watching may have changed unseen, so the webview's
    // cached copy cannot be trusted; a bump refetches it. A watched one is left
    // alone — that is what keeps a refresh's scroll restore honest, since the
    // page then has its pictures' full height straight away.
    if (isImage(file) && !isWatched(file)) bumpAsset(file);
    el.setAttribute("src", assetUrl(file));
    // Remember the file behind the picture so a click can open it full size —
    // a diagram at column width is often too small to read. Images only: the
    // same loop also rewrites video and audio, which have their own controls.
    if (el.tagName === "IMG" && isImage(file)) el.dataset.file = file;
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
  els.image.hidden = which !== "image";
  els.error.hidden = which !== "error";
  // The image panel brings its own scroll box. Left to itself the body would
  // scroll too, giving two scrollbars for one thing to scroll.
  document.documentElement.classList.toggle("image-mode", which === "image");
  // Anything else on screen means there is no picture to zoom, and every
  // handler that reads `picture` checks it first.
  if (which !== "image") picture = null;
}

/*
 * Drop any fragment left over from the previous document. Without this, a `#f5`
 * still sitting in the URL means clicking `#f5` in the *next* file is not a
 * change of fragment, so the browser performs no jump at all. replaceState
 * rather than assigning location.hash: no extra entry, no hashchange, no
 * trailing "#".
 */
function clearHash() {
  if (location.hash) {
    history.replaceState(null, "", location.href.split("#")[0]);
  }
}

function renderDocument(doc, scrollY, hash) {
  clearHash();

  els.content.innerHTML = doc.html;
  resolveMedia(els.content, doc.dir);
  wrapTables(els.content);
  highlight(els.content);
  show("content");

  // Restore after layout, so the offset being scrolled to actually exists yet.
  requestAnimationFrame(() => {
    /*
     * Arriving through a cross-file link: land on the section it named. Only
     * on arrival — `scrollY` is zero exactly when nothing has been recorded
     * yet, and once this entry has a position of its own, Back and Forward
     * must return to that rather than jumping to the anchor a second time.
     */
    if (hash && scrollY === 0 && jumpToAnchor(hash)) {
      requestAnimationFrame(() => {
        const entry = currentEntry(activeTab());
        if (entry?.hash === hash) entry.scrollY = window.scrollY;
      });
      return;
    }
    window.scrollTo(0, scrollY);
  });
}

/* ---------------- image viewer ---------------- */

/*
 * A diagram is often far wider than the window — the ERDs this was written for
 * are 10:1 — so it gets a scroll box of its own rather than the body scroll a
 * document uses, and a zoom that can go well past the window's width.
 *
 * Zoom is an explicit pixel width on the image, never a CSS transform. A
 * transform paints outside the layout, so the scroll box would not know the
 * picture had grown and there would be nothing to scroll; a width is real
 * layout, and the scrollbars follow from it for free.
 */

const ZOOM_STEP = 1.25;
const ZOOM_MIN = 0.05;
const ZOOM_MAX = 32;
const PAN_THRESHOLD = 3; // px before a click on the picture becomes a pan

/**
 * The picture on screen, or null whenever another panel is up. `base` is the
 * width 100% refers to, `fit` records that the size is the window's to choose
 * rather than one the reader picked.
 */
let picture = null;

/**
 * Width that shows the whole picture, whichever way round it is. Measured
 * against the panel rather than the scroll box inside it: a picture that fits
 * needs no scrollbars, so the space they are taking up right now is space the
 * fitted picture will have back, and measuring around them fits it too small.
 */
function fitWidth() {
  const w = els.image.clientWidth;
  const h = els.image.clientHeight;
  return Math.max(1, Math.min(w, h * picture.ratio));
}

/**
 * The width 100% means. A raster image has a true pixel size to be honest
 * about; an SVG with only a viewBox has none, so there "100%" is what fits —
 * which also makes Fit read as 100%, the more useful reading of the two.
 */
function baseWidth() {
  return picture.isRaster ? picture.naturalW : fitWidth();
}

/** Where an untouched picture starts: filling the window, but never blown up. */
function defaultWidth() {
  return picture.isRaster ? Math.min(picture.naturalW, fitWidth()) : fitWidth();
}

function applyWidth(w) {
  const base = baseWidth();
  const width = Math.min(base * ZOOM_MAX, Math.max(base * ZOOM_MIN, w));
  picture.base = base;
  picture.width = width;
  els.imageEl.style.width = `${width}px`;
  els.zoomLevel.textContent = `${Math.round((width / base) * 100)}%`;
}

/**
 * Resize about a point, so whatever was under the cursor stays under it. Done
 * by measuring the picture before and after rather than by arithmetic on
 * offsets: the image is centred while it is smaller than the box and hard
 * against the edge once it is bigger, and measuring is right either way.
 */
function zoomTo(w, clientX, clientY) {
  if (!picture) return;
  const box = els.imageView.getBoundingClientRect();
  const before = els.imageEl.getBoundingClientRect();
  const ax = clientX ?? box.left + box.width / 2;
  const ay = clientY ?? box.top + box.height / 2;
  const fx = before.width ? (ax - before.left) / before.width : 0.5;
  const fy = before.height ? (ay - before.top) / before.height : 0.5;

  applyWidth(w);

  const after = els.imageEl.getBoundingClientRect();
  els.imageView.scrollLeft += after.left + fx * after.width - ax;
  els.imageView.scrollTop += after.top + fy * after.height - ay;
  rememberImage();
}

function zoomBy(factor, clientX, clientY) {
  if (!picture) return;
  picture.fit = false;
  zoomTo(picture.width * factor, clientX, clientY);
}

function fitImage() {
  if (!picture) return;
  picture.fit = true;
  zoomTo(defaultWidth());
  els.imageView.scrollLeft = 0;
  els.imageView.scrollTop = 0;
  rememberImage();
}

function actualSize() {
  if (!picture) return;
  picture.fit = false;
  zoomTo(picture.naturalW);
}

/** Bank zoom and pan on the history entry, so a tab switch returns to them. */
function rememberImage() {
  const entry = currentEntry(activeTab());
  if (!entry || !picture) return;
  entry.scale = picture.fit ? null : picture.width / picture.base;
  entry.scrollLeft = els.imageView.scrollLeft;
  entry.scrollTop = els.imageView.scrollTop;
}

async function showImage(asset, entry, token) {
  // The strip may have just appeared or gone; the panel is sized against the
  // bar, so settle its height before anything is measured against it.
  renderTabs();

  els.imageEl.style.width = "";
  els.zoomLevel.textContent = "";
  els.imageEl.alt = baseName(asset.path);
  els.imageEl.src = assetUrl(asset.path);
  clearHash();
  show("image");

  // Nothing can be measured until it has decoded, and a rejection here is a
  // file that has gone or will not parse — which the error panel exists for.
  await els.imageEl.decode();
  if (token !== renderToken) return; // a newer switch already won

  const { naturalWidth: nw, naturalHeight: nh } = els.imageEl;
  picture = {
    // The ratio is trustworthy even when the size is not: an SVG sized only by
    // a viewBox reports some arbitrary box scaled to the right shape.
    ratio: nh ? nw / nh : 1,
    isRaster: !/\.svg$/i.test(asset.path),
    naturalW: nw || 1,
    width: 0,
    base: 1,
    fit: entry.scale == null,
  };
  // Meaningless for a picture that has no true size of its own.
  els.imageTools.querySelector('[data-zoom="actual"]').hidden = !picture.isRaster;

  applyWidth(picture.fit ? defaultWidth() : entry.scale * baseWidth());
  els.imageView.scrollLeft = entry.scrollLeft ?? 0;
  els.imageView.scrollTop = entry.scrollTop ?? 0;
}

/**
 * A resize changes the panel, and with it what "fits". A picture the reader
 * sized keeps its zoom — which for an SVG, whose 100% is the fit, means it
 * grows and shrinks with the window rather than sitting at a stale width.
 */
function onResize() {
  measureBar();
  if (!picture) return;
  const scale = picture.base ? picture.width / picture.base : 1;
  zoomTo(picture.fit ? defaultWidth() : scale * baseWidth());
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
    if (isImage(entry.path)) {
      // No content to fetch: the webview loads the bytes itself over the asset
      // protocol. What this call is for is the permission to do so.
      const asset = await invoke("load_asset", { path: entry.path });
      if (token !== renderToken) return; // a newer switch already won
      entry.path = asset.path;
      tab.path = asset.path;
      tab.dir = asset.dir;
      tab.label = baseName(asset.path);
      tab.heading = "";
      tab.media = []; // a picture shows only itself
      // `scrollY` is a document position and means nothing here; the picture
      // restores its own zoom and pan from the entry.
      //
      // Nothing was watching this file, so the copy the webview holds may be
      // stale: Back and Forward, a reopened tab, and a picture opened in its
      // own tab after being embedded all land here and refetch.
      if (!isWatched(asset.path)) bumpAsset(asset.path);
      await showImage(asset, entry, token);
      if (token !== renderToken) return;
    } else {
      const doc = await invoke("load_file", { path: entry.path });
      if (token !== renderToken) return; // a newer switch already won
      entry.path = doc.path;
      tab.path = doc.path;
      tab.dir = doc.dir;
      tab.label = baseName(doc.path);
      tab.heading = doc.title ?? "";
      renderDocument(doc, scrollY ?? entry.scrollY ?? 0, entry.hash);
      // Only a rendered document has pictures the webview can be holding stale;
      // recording them here covers exactly those, and the list survives a tab
      // switch, so a document in the background stays watched.
      tab.media = [
        ...new Set([...els.content.querySelectorAll("img[data-file]")].map((i) => i.dataset.file)),
      ];
    }
  } catch (err) {
    if (token !== renderToken) return;
    // An error panel shows no pictures, so the watcher should not go on
    // holding the previous document's.
    tab.media = [];
    els.errorDetail.textContent = String(err);
    show("error");
  }
  syncWatch();
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
  markTreeSelection();
  appWindow
    .setTitle(tab ? `${tab.label} — Markdown Viewer` : "Markdown Viewer")
    .catch(() => {});
  renderTabs();
}

/**
 * Publish the bar's height. A document scrolls the body and simply flows under
 * the sticky bar, but the image panel has to be exactly the leftover height or
 * its scroll box is the wrong size — and the bar grows a row the moment a
 * second tab opens, so no constant will do.
 */
function measureBar() {
  const h = els.bar.getBoundingClientRect().height;
  document.documentElement.style.setProperty("--bar-h", `${h}px`);
}

/* ---------------- tab strip ---------------- */

/** Slot the drop caret sits in, or -1 when no drag is hovering this window. */
let dropCaret = -1;

function renderTabs() {
  const visible = tabs.length > 1 || dropCaret >= 0;
  els.tabs.hidden = !visible;
  if (!visible) {
    els.tabs.replaceChildren();
    measureBar();
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
  measureBar();
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
/** The same paths in comparison form, so `isWatched` need not rebuild them. */
const watched = new Set();

/** Keep the Rust watcher pointed at exactly the files this window has open. */
function syncWatch() {
  // A document's pictures go in the list too: a re-saved diagram has to reach the
  // page it is drawn on, not just the page's own file.
  const paths = [
    ...new Set(tabs.flatMap((t) => [currentEntry(t)?.path, ...t.media]).filter(Boolean)),
  ];
  const key = paths.join("\0");
  if (key === watching) return;
  watching = key;
  watched.clear();
  for (const p of paths) watched.add(normPath(p));
  invoke("watch_files", { paths }).catch(console.error);
}

/** Whether a change to this file would be reported, or would pass unnoticed. */
function isWatched(file) {
  return watched.has(normPath(file));
}

/**
 * Answer Rust's `update-installing` with what this window has open, so the
 * restart can bring it back. The same shape a tab travels in between windows.
 */
function reportSession() {
  return invoke("set_session", {
    tabs: tabs.map(packTab),
    active: Math.max(0, tabs.findIndex((t) => t.id === activeId)),
  }).catch(console.error);
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

/** The plain JSON a tab is handed around as: between windows, and across an update restart. */
function packTab(tab) {
  return { path: tab.path, entries: tab.entries, index: tab.index };
}

/** A live tab from the plain JSON one is handed around as, or null if it names nothing. */
function rebuildTab(data) {
  const entries =
    Array.isArray(data?.entries) && data.entries.length
      ? data.entries
      : [{ path: data?.path, scrollY: 0 }];
  const index = Math.max(0, Math.min(entries.length - 1, data?.index ?? 0));
  const path = entries[index]?.path;
  if (!path) return null;
  return Object.assign(makeTab(path), { entries, index });
}

/** Rebuild a tab handed over from another window and take it on. */
async function adoptTab(data, at) {
  const tab = rebuildTab(data);
  if (!tab) return;

  rememberScroll();
  tabs.splice(Math.max(0, Math.min(tabs.length, at)), 0, tab);
  activeId = tab.id;
  syncWatch();
  await showActive();
  appWindow.setFocus().catch(() => {});
}

/** Put back every tab an update restart carried over, in order. */
async function restoreTabs(list, active) {
  tabs = list.map(rebuildTab).filter(Boolean);
  activeId = (tabs[active] ?? tabs[0])?.id ?? null;
  syncWatch();
  await showActive();
}

/* ---------------- folder sidebar ---------------- */

/** Per-list render tokens: a watcher burst and a click can re-list the same folder. */
const treeTokens = new WeakMap();
/** What each list last showed, so a save that changes nothing does not rebuild it. */
const treeListings = new WeakMap();

/**
 * One level at a time. A list is rebuilt whenever its folder is expanded or
 * the watcher reports a change in it, and whatever was expanded inside it is
 * expanded again afterwards, so a re-list never costs the user their place.
 */
async function renderTree(ul, dir) {
  const token = (treeTokens.get(ul) ?? 0) + 1;
  treeTokens.set(ul, token);
  ul.dataset.dir = dir;

  let listing;
  try {
    listing = await invoke("list_dir", { path: dir });
  } catch (err) {
    if (token !== treeTokens.get(ul)) return;
    treeListings.delete(ul);
    const li = document.createElement("li");
    li.className = "tree-row error";
    li.textContent = String(err);
    ul.replaceChildren(li);
    return;
  }
  if (token !== treeTokens.get(ul)) return; // a newer listing already won
  ul.dataset.dir = listing.dir;
  const { entries } = listing;

  // The watcher reports every save in the folder, and a save changes nothing
  // the tree shows. Rebuilding anyway would collapse-and-reopen the subtree.
  const signature = JSON.stringify(entries);
  if (signature === treeListings.get(ul)) return;
  treeListings.set(ul, signature);

  const expanded = new Set(
    [...ul.querySelectorAll(':scope > li > .tree-row[aria-expanded="true"]')].map((r) => r.dataset.path),
  );

  const nodes = entries.map((e) => {
    const li = document.createElement("li");
    li.setAttribute("role", "treeitem");

    const row = document.createElement("div");
    row.className = "tree-row";
    row.dataset.path = e.path;
    row.title = e.path;

    const twist = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    twist.setAttribute("class", "tree-twist");
    twist.setAttribute("viewBox", "0 0 16 16");
    twist.setAttribute("aria-hidden", "true");
    if (e.is_dir) {
      const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
      path.setAttribute("d", "M6 3.5 L10.5 8 L6 12.5");
      twist.append(path);
      row.dataset.dir = "1";
      row.setAttribute("aria-expanded", "false");
    }

    const name = document.createElement("span");
    name.className = "tree-name";
    name.textContent = e.name;

    row.append(twist, name);
    li.append(row);
    if (e.is_dir) {
      const children = document.createElement("ul");
      children.setAttribute("role", "group");
      children.hidden = true;
      li.append(children);
    }
    return li;
  });
  ul.replaceChildren(...nodes);
  markTreeSelection();

  const reopen = [...ul.querySelectorAll(":scope > li > .tree-row[data-dir]")].filter((r) =>
    expanded.has(r.dataset.path),
  );
  await Promise.all(reopen.map((r) => expandRow(r, true)));
}

/** Open or close a folder row. Leaves the watcher alone — see syncFolderWatch. */
async function expandRow(row, open) {
  row.setAttribute("aria-expanded", String(open));
  const children = row.nextElementSibling;
  children.hidden = !open;
  if (open) await renderTree(children, row.dataset.path);
}

let watchingFolders = null;

/**
 * Keep the Rust watcher on exactly the folders on show: the root and every
 * expanded row that is not itself inside a collapsed one. Called once a change
 * to the tree has settled, never from the middle of a rebuild — a half-built
 * list would look emptier than it is, and dropping a watch to re-add it a
 * moment later loses whatever happened in between.
 */
function syncFolderWatch() {
  const dirs = [];
  if (state.folder !== null) {
    dirs.push(state.folder);
    for (const row of els.tree.querySelectorAll('.tree-row[aria-expanded="true"]')) {
      if (!row.closest("ul[hidden]")) dirs.push(row.dataset.path);
    }
  }
  const key = dirs.join("\0");
  if (key === watchingFolders) return;
  watchingFolders = key;
  invoke("watch_folders", { dirs }).catch(console.error);
}

async function openFolder(path) {
  state.folder = path;
  els.sidebar.hidden = false;
  treeListings.delete(els.tree); // a different folder must rebuild even if it lists the same
  await renderTree(els.tree, path);
  // Canonical from here on, so it compares with what the watcher reports.
  if (state.folder === path) state.folder = els.tree.dataset.dir;
  els.sidebarName.textContent = baseName(state.folder);
  els.sidebarName.title = state.folder;
  syncFolderWatch();
}

function closeFolder() {
  state.folder = null;
  els.sidebar.hidden = true;
  els.tree.replaceChildren();
  syncFolderWatch();
}

/** `samePath` for folders: the picker may hand back a trailing separator. */
function sameDir(a, b) {
  const trim = (p) => p.replace(/[\\/]+$/, "");
  return samePath(trim(a), trim(b));
}

/** The watcher saw something move; re-list each affected folder that is on show. */
async function onFolderChanged(paths) {
  if (state.folder === null) return;
  const dirs = [...new Set(paths.map(dirName))];
  const lists = [els.tree, ...els.tree.querySelectorAll("ul")];
  await Promise.all(
    dirs.map((dir) => {
      const ul = lists.find((l) => l.dataset.dir && !l.closest("ul[hidden]") && sameDir(l.dataset.dir, dir));
      return ul ? renderTree(ul, ul.dataset.dir) : null;
    }),
  );
  syncFolderWatch();
}

/** Light up the row for the document on screen, if the tree shows it. */
function markTreeSelection() {
  if (state.folder === null) return;
  const path = currentEntry(activeTab())?.path;
  for (const row of els.tree.querySelectorAll(".tree-row[data-path]")) {
    row.classList.toggle("active", !!path && !row.dataset.dir && samePath(row.dataset.path, path));
  }
}

async function onTreeClick(event) {
  const row = event.target.closest(".tree-row[data-path]");
  if (!row) return;
  const path = row.dataset.path;

  if (row.dataset.dir) {
    await expandRow(row, row.getAttribute("aria-expanded") !== "true");
    syncFolderWatch();
    return;
  }

  // Ctrl+click opens beside the current document rather than in its place, as
  // in a browser. Plain click walks the active tab's history like a link.
  if (event.ctrlKey || event.metaKey) await openTab(path);
  else await loadPath(path);
}

/** Middle click never fires `click`; it means "new tab" here as in a browser. */
async function onTreeAuxClick(event) {
  if (event.button !== 1) return;
  const row = event.target.closest(".tree-row[data-path]");
  if (!row || row.dataset.dir) return;
  event.preventDefault();
  await openTab(row.dataset.path);
}

/* ---------------- navigation ---------------- */

/**
 * Follow a link in the active tab, discarding any forward history. `hash` is
 * the fragment the link carried, if any, and is stored decoded so it can be
 * matched against ids straight out of the DOM.
 */
async function loadPath(path, hash) {
  const tab = activeTab();
  if (!tab) return openTab(path);

  const id = hash ? decodeId(hash) : "";
  const entry = currentEntry(tab);

  // A cross-file link can name the file it is written in — documents that link
  // to their own sections by full name do it constantly. Loading it again would
  // throw away the rendered page to arrive at the same one, so treat it as the
  // in-page jump it really is.
  if (id && entry && samePath(entry.path, path)) {
    pushAnchorEntry(id);
    jumpToAnchor(id);
    return;
  }

  if (!entry || !samePath(entry.path, path)) {
    rememberScroll();
    tab.entries.length = tab.index + 1; // drop the forward branch
    tab.entries.push({ path, scrollY: 0, hash: id });
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

/** Fragments arrive percent-encoded; the ids they name do not. */
function decodeId(raw) {
  try {
    return decodeURIComponent(raw);
  } catch {
    return raw; // malformed escapes: use the raw text
  }
}

/**
 * Jump the way a click on a same-page link would, rather than scrolling by
 * hand: that is what makes `:target` match, and `:target` is what keeps the
 * heading clear of the sticky bar. False when this document has no such id,
 * which is all a link into a section that has since been renamed deserves.
 */
function jumpToAnchor(id) {
  if (!document.getElementById(id)) return false;
  location.hash = id;
  return true;
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
  const id = decodeId(raw);
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
  const entry = currentEntry(activeTab());
  if (!entry) return;
  // Bank the position as well as restoring it, so a later tab switch or Back
  // returns here rather than to wherever the entry was last left.
  rememberScroll();
  // A re-saved picture keeps its path, and the webview would serve the copy it
  // already has. Documents are re-read by Rust, and their pictures are watched
  // in their own right, so nothing else needs invalidating here — refetching
  // them would leave the page short of their height when the scroll is restored.
  if (isImage(entry.path)) bumpAsset(entry.path);
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
    mode === "window" ? "Open a file in a new window (Ctrl+O)" : "Open a file in a new tab (Ctrl+O)";
}

function setOpenMode(mode) {
  showOpenMode(mode);
  invoke("set_open_mode", { mode }).catch(console.error);
}

/* ---------------- updates ---------------- */

/**
 * Ask Rust whether a newer release exists. Rust caches the answer, so the
 * second and third window cost nothing.
 *
 * The boot call passes `force: false`: it obeys the Settings switch, and a
 * failure — offline, GitHub down — is swallowed, because an update check the
 * user never asked for has no business interrupting them. `force: true` comes
 * from the Check now button, which does want to hear about failures.
 */
async function checkUpdate(force) {
  const info = await invoke("check_for_update", { force });
  state.update = info ?? null;
  els.updateBtn.hidden = !info;
  if (info) els.updateBtn.title = `Version ${info.version} is available`;
  return info;
}

function showUpdateDialog() {
  const info = state.update;
  if (!info) return;

  els.updateSummary.textContent = `Version ${info.version} is available.`;
  els.updateNotes.textContent = info.notes;
  els.updateNotes.hidden = !info.notes;
  els.updateProgress.hidden = true;
  els.updateError.hidden = true;
  els.updateNow.disabled = false;

  // A deb or rpm install cannot replace itself; that is the package manager's
  // job. Offering an Update button that could only ever fail would be worse
  // than sending them to the download page.
  els.updateNow.textContent = info.installable ? "Update now" : "Download…";
  els.updateWarning.hidden = !info.installable;

  els.updateDialog.showModal();
}

async function runUpdate() {
  const info = state.update;
  if (!info) return;

  if (!info.installable) {
    openUrl(info.release_url).catch(console.error);
    return;
  }

  els.updateNow.disabled = true;
  els.updateError.hidden = true;
  els.updateProgress.hidden = false;
  els.updateProgress.textContent = "Downloading…";

  try {
    // Does not return when it succeeds: the app is restarted into the new
    // version, or on Windows killed outright by the installer.
    await invoke("install_update");
  } catch (err) {
    console.error(err);
    els.updateProgress.hidden = true;
    els.updateError.textContent = `Update failed: ${err}`;
    els.updateError.hidden = false;
    els.updateNow.disabled = false;
  }
}

/** `null` percent means the manifest gave no size to measure against. */
function showUpdateProgress(percent) {
  if (els.updateProgress.hidden) return;
  els.updateProgress.textContent =
    percent == null ? "Downloading…" : `Downloading… ${percent}%`;
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
 *
 * Wayland will not say where a window is, so `origin.exact` is false there and
 * the guess is all there is. It only decides where a torn-off window appears,
 * and being a little off beats the drag doing nothing.
 */
function screenPoint(event, origin) {
  if (!origin.exact) {
    return {
      x: event.screenX * window.devicePixelRatio,
      y: event.screenY * window.devicePixelRatio,
    };
  }
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

  // Off Windows nothing can tell which window is under the cursor, so probing
  // would only ever answer "none". Skipping it means no other window lights up
  // a drop caret, which is the honest signal: releasing here tears the tab off.
  if (!state.crossWindowDrag) return;

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
  // Never bail for want of an origin: the tab has already been lifted out of the
  // strip, so giving up here would strand it. An inexact origin sends
  // `screenPoint` down its fallback, which is good enough to place a new window.
  const origin =
    d.origin ??
    (await invoke("window_origin").catch(() => null)) ??
    { x: 0, y: 0, scale: window.devicePixelRatio, exact: false };
  const { x, y } = screenPoint(event, origin);
  try {
    const outcome = await invoke("drop_tab", {
      x,
      y,
      tab: packTab(tab),
      // The last tab already has a window to itself: tearing it off would only
      // swap this window for a new one and leave an empty shell behind.
      tearOff: tabs.length > 1,
    });
    // "adopted" — another window took it. "detached" — it became a new window.
    // "cancelled" — nowhere to go; the tab stays put.
    if (outcome === "cancelled") {
      renderTabs();
      return;
    }
    if (outcome === "adopted" || outcome === "detached") await removeTab(d.id);
    // Handing the last tab to another window leaves nothing here worth keeping;
    // the user is already looking at the target, so close the empty shell.
    if (outcome === "adopted" && tabs.length === 0) await appWindow.close();
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
      { name: "Images", extensions: ["svg", "png", "jpg", "jpeg", "gif", "webp", "avif", "bmp", "ico"] },
      { name: "All files", extensions: ["*"] },
    ],
  });
  return typeof picked === "string" ? picked : null;
}

async function pickFolder() {
  const picked = await openDialog({ directory: true, multiple: false });
  return typeof picked === "string" ? picked : null;
}

/** The active document's folder, or "" when there is none to name. */
function activeDir() {
  const tab = activeTab();
  if (!tab) return "";
  // Before the first load finishes (or after a failed one) `dir` is empty.
  return tab.dir || dirName(tab.path);
}

/**
 * Toggle the sidebar. Opening shows the folder of what is on screen — almost
 * always the one wanted — and asks only when nothing is open.
 */
async function showFolder() {
  if (state.folder !== null) {
    closeFolder();
    return;
  }
  const path = activeDir() || (await pickFolder());
  if (path) await openFolder(path);
}

/**
 * The webview is the whole app: letting it navigate away would leave a dead
 * window. Intercept every link — follow anchors, open sibling documents in
 * place, hand everything else to the system browser.
 */
function onLinkClick(event) {
  const a = event.target.closest("a[href]");
  if (!a) {
    // A picture in a document is held to the column width, which is no width at
    // all for a wide diagram. Clicking one opens it where it can be read.
    // Only outside a link: a linked image still means the link.
    const img = event.target.closest("img[data-file]");
    if (img) {
      event.preventDefault();
      openTab(img.dataset.file).catch(console.error);
    }
    return;
  }
  const href = a.getAttribute("href");
  if (!href) return;

  // In-page anchor: the browser performs the jump, we just record it.
  if (href.startsWith("#")) {
    pushAnchorEntry(href.slice(1));
    return;
  }

  event.preventDefault();

  if (isRelative(href)) {
    const [pathPart, ...rest] = href.split("#");
    const target = resolvePath(activeTab()?.dir ?? "", pathPart);
    if (MD_LINK.test(pathPart)) {
      // `notes.md#fc-29` is one link, not two: the file to load and the place
      // in it to land. Dropping the fragment would open every cross-file
      // reference at the top of its document.
      loadPath(target, rest.join("#"));
    } else if (isImage(pathPart)) {
      // A new tab, not this one: the document that linked the diagram is the
      // thing you were reading, and closing the tab is how you get back to it.
      openTab(target).catch(console.error);
    } else {
      openUrl(convertFileSrc(target)).catch(console.error);
    }
    return;
  }

  openUrl(href).catch(console.error);
}

/*
 * Panning the picture. Pointer capture rather than a document-level listener so
 * a drag that leaves the window still steers the scroll, and so releasing
 * outside it still ends cleanly.
 */
let pan = null;

function onImagePointerDown(event) {
  if (!picture || event.button !== 0) return;
  // Also what stops the browser starting its own image drag instead.
  event.preventDefault();
  pan = { pointerId: event.pointerId, x: event.clientX, y: event.clientY, moved: false };
  els.imageView.setPointerCapture(event.pointerId);
}

function onImagePointerMove(event) {
  if (!pan || event.pointerId !== pan.pointerId) return;
  const dx = event.clientX - pan.x;
  const dy = event.clientY - pan.y;
  if (!pan.moved && Math.hypot(dx, dy) < PAN_THRESHOLD) return;

  pan.moved = true;
  pan.x = event.clientX;
  pan.y = event.clientY;
  els.imageView.classList.add("panning");
  // Dragging the picture left means looking further right.
  els.imageView.scrollLeft -= dx;
  els.imageView.scrollTop -= dy;
}

function onImagePointerUp(event) {
  if (!pan || event.pointerId !== pan.pointerId) return;
  const moved = pan.moved;
  pan = null;
  try {
    els.imageView.releasePointerCapture(event.pointerId);
  } catch {
    /* capture already gone */
  }
  els.imageView.classList.remove("panning");
  if (moved) rememberImage();
}

/**
 * Plain and Shift wheel are left alone: the scroll box already handles them,
 * vertically and horizontally. Ctrl is the zoom, as everywhere else.
 */
function onImageWheel(event) {
  if (!picture || !event.ctrlKey) return;
  event.preventDefault();
  // Some wheels report lines rather than pixels; scale them to the same feel.
  const dy = event.deltaMode === 1 ? event.deltaY * 16 : event.deltaY;
  zoomBy(Math.exp(-dy * 0.002), event.clientX, event.clientY);
}

/** The usual image-viewer double-click: in on what you pointed at, then back. */
function onImageDblClick(event) {
  if (!picture) return;
  if (picture.fit) zoomBy(2.5, event.clientX, event.clientY);
  else fitImage();
}

function onZoomClick(event) {
  const button = event.target.closest("button[data-zoom]");
  if (!button) return;
  const what = button.dataset.zoom;
  if (what === "in") zoomBy(ZOOM_STEP);
  else if (what === "out") zoomBy(1 / ZOOM_STEP);
  else if (what === "fit") fitImage();
  else if (what === "actual") actualSize();
}

async function onKeydown(event) {
  if (event.key === "F8") {
    event.preventDefault();
    cycleTheme(event.shiftKey ? -1 : 1);
    return;
  }

  /*
   * F5 means "re-read this document", not "reload the webview". Left to the
   * browser it reloads index.html, which restarts the app — and since the
   * pending file was consumed at boot, that lands on the empty state having
   * thrown away every tab and its history. Ctrl+R is the same action, handled
   * below; both suppress the default.
   *
   * Handled before the modal guard, and with any modifier, so no spelling of
   * a reload key can get past it — Ctrl+F5 and Shift+F5 included.
   */
  if (event.key === "F5") {
    event.preventDefault();
    refresh();
    return;
  }

  // Otherwise a dialog is modal: let it own the keyboard, Escape included.
  if (els.settings.open || els.updateDialog.open) return;

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

  // Alt+Arrow above is the Windows idiom; Cmd+[ and Cmd+] are the Mac one.
  // Both are accepted everywhere rather than branching on the platform.
  if (event.key === "[") {
    event.preventDefault();
    go(-1);
    return;
  }
  if (event.key === "]") {
    event.preventDefault();
    go(1);
    return;
  }

  if (event.key === "Tab") {
    event.preventDefault();
    cycleTab(event.shiftKey ? -1 : 1);
    return;
  }

  // The browser spellings of zoom, pointed at the picture rather than the page.
  // `=` is the unshifted key `+` lives on, which is how it is usually pressed.
  if (picture && (event.key === "+" || event.key === "=")) {
    event.preventDefault();
    zoomBy(ZOOM_STEP);
    return;
  }
  if (picture && event.key === "-") {
    event.preventDefault();
    zoomBy(1 / ZOOM_STEP);
    return;
  }
  if (picture && event.key === "0") {
    event.preventDefault();
    fitImage();
    return;
  }

  const key = event.key.toLowerCase();
  if (key === "o" && event.shiftKey) {
    event.preventDefault();
    await showFolder();
  } else if (key === "o") {
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

  els.folderBtn.addEventListener("click", async () => {
    showOpenMenu(false);
    await showFolder();
  });
  els.sidebarClose.addEventListener("click", closeFolder);
  els.sidebarName.addEventListener("click", () => {
    if (state.folder !== null) {
      treeListings.delete(els.tree); // an explicit refresh always rebuilds
      renderTree(els.tree, state.folder).then(syncFolderWatch).catch(console.error);
    }
  });
  els.tree.addEventListener("click", (e) => onTreeClick(e).catch(console.error));
  els.tree.addEventListener("auxclick", (e) => onTreeAuxClick(e).catch(console.error));

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

  els.autoUpdate.addEventListener("change", () => {
    invoke("set_auto_update_check", { enabled: els.autoUpdate.checked }).catch(console.error);
  });
  els.checkNow.addEventListener("click", async () => {
    els.checkNow.disabled = true;
    els.updateStatus.textContent = "Checking…";
    try {
      const info = await checkUpdate(true);
      els.updateStatus.textContent = info
        ? `Version ${info.version} is available.`
        : "You are up to date.";
    } catch (err) {
      console.error(err);
      els.updateStatus.textContent = `Check failed: ${err}`;
    } finally {
      els.checkNow.disabled = false;
    }
  });

  els.updateBtn.addEventListener("click", () => {
    showOpenMenu(false);
    showUpdateDialog();
  });
  els.updateNow.addEventListener("click", runUpdate);
  els.updateNotesBtn.addEventListener("click", () => {
    if (state.update) openUrl(state.update.release_url).catch(console.error);
  });

  els.picker.addEventListener("change", (e) => selectTheme(e.target.value));
  els.content.addEventListener("click", onLinkClick);
  els.back.addEventListener("click", () => go(-1));
  els.forward.addEventListener("click", () => go(1));

  // Not passive: preventDefault is what keeps a Ctrl+wheel zoom from scrolling
  // the box at the same time, and a passive listener is not allowed to.
  els.imageView.addEventListener("wheel", onImageWheel, { passive: false });
  els.imageView.addEventListener("pointerdown", onImagePointerDown);
  els.imageView.addEventListener("pointermove", onImagePointerMove);
  els.imageView.addEventListener("pointerup", onImagePointerUp);
  els.imageView.addEventListener("pointercancel", onImagePointerUp);
  els.imageView.addEventListener("dblclick", onImageDblClick);
  els.imageTools.addEventListener("click", onZoomClick);
  window.addEventListener("resize", onResize);

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
  state.crossWindowDrag = settings.cross_window_drag === true;
  state.caseInsensitivePaths = settings.case_insensitive_paths === true;
  els.autoUpdate.checked = settings.auto_update_check !== false;
  els.appVersion.textContent = settings.version ?? "";
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
  await listenHere("folder-changed", (e) => onFolderChanged(e.payload ?? []).catch(console.error));
  await listenHere("file-changed", async (e) => {
    const changed = e.payload ?? [];
    const open = currentEntry(activeTab())?.path;
    if (open && changed.some((c) => samePath(c, open))) {
      // Re-render with the pictures the webview already holds, so the page has
      // its full height when the scroll is restored; new bytes go in below.
      await refresh();
      // The restore is queued in renderDocument's rAF; run after it.
      await new Promise(requestAnimationFrame);
    }
    // A picture the webview has fetched once is served from its cache the next
    // time too, so every changed image needs a new version whether or not a tab
    // showing it is the one on screen — the background tab reads it on the way in.
    for (const p of changed) if (isImage(p)) bumpAsset(p);
    // A picture embedded in the document on screen: swap that one `<img>` rather
    // than re-rendering the page around it, which would cost the reading position.
    if (!els.content.hidden) {
      for (const img of els.content.querySelectorAll("img[data-file]")) {
        const file = img.dataset.file;
        if (changed.some((c) => samePath(c, file))) img.src = assetUrl(file);
      }
    }
  });
  await listen("themes-changed", async () => {
    await loadThemeList();
    if (state.theme) await applyTheme(state.theme);
  });
  await listen("open-mode-changed", (e) => showOpenMode(e.payload));
  // Broadcast on purpose: one install is happening to the whole app, so every
  // window's dialog should count along with it.
  await listen("update-progress", (e) => showUpdateProgress(e.payload));
  // Rust asks once the update is downloaded and waits for the answer. The
  // reader's place is only noted when they leave a document, so the restart
  // would otherwise land them where they last switched tabs.
  await listen("update-installing", () => {
    rememberScroll();
    reportSession();
  });

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

  // Whatever this window was created to show: a file-association open, a tab
  // torn off another window, or everything it had before an update restart.
  const pending = await invoke("take_pending");
  if (pending?.kind === "path") {
    await openTab(pending.path);
  } else if (pending?.kind === "tab") {
    await adoptTab(pending.tab, 0);
  } else if (pending?.kind === "session") {
    await restoreTabs(pending.tabs, pending.active);
  } else {
    show("empty");
    updateChrome();
  }

  // Window starts hidden so the first frame is already themed and painted. A
  // restored maximize waits for the same reason: on Windows it shows the window.
  if (pending?.maximized) await appWindow.maximize().catch(() => {});
  await appWindow.show();
  // Showing does not raise a window whose process is in the background, and a
  // window created for a file the user just opened has to land in front.
  await appWindow.setFocus().catch(() => {});

  // Last, and not awaited: the document is already on screen, and a slow or
  // unreachable GitHub must cost the reader nothing.
  checkUpdate(false).catch(console.error);
}

main().catch((err) => {
  console.error(err);
  els.errorDetail.textContent = String(err);
  show("error");
  appWindow.show();
});
