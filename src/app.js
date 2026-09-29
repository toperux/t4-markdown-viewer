"use strict";

const { invoke, convertFileSrc } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { open: openNativeDialog } = window.__TAURI__.dialog;
const { openUrl } = window.__TAURI__.opener;
const appWindow = window.__TAURI__.window.getCurrentWindow();

const els = {
  bar: document.getElementById("bar"),
  back: document.getElementById("back-btn"),
  forward: document.getElementById("fwd-btn"),
  docName: document.getElementById("doc-name"),
  tabs: document.getElementById("tabs"),
  picker: document.getElementById("theme-picker"),
  themeToggle: document.getElementById("theme-mode-btn"),
  themeSun: document.getElementById("theme-mode-sun"),
  themeMoon: document.getElementById("theme-mode-moon"),
  openBtn: document.getElementById("open-btn"),
  openMore: document.getElementById("open-more"),
  openMenu: document.getElementById("open-menu"),
  folderBtn: document.getElementById("folder-btn"),
  sidebar: document.getElementById("sidebar"),
  sidebarName: document.getElementById("sidebar-name"),
  sidebarNameText: document.getElementById("sidebar-name-text"),
  sidebarSort: document.getElementById("sidebar-sort"),
  sidebarClose: document.getElementById("sidebar-close"),
  treeFilter: document.getElementById("tree-filter"),
  tree: document.getElementById("tree"),
  treeMenu: document.getElementById("tree-menu"),
  sortMenu: document.getElementById("sort-menu"),
  settingsBtn: document.getElementById("settings-btn"),
  settings: document.getElementById("settings-dialog"),
  modeRadios: document.querySelectorAll('#settings-dialog input[name="open-mode"]'),
  reopenRadios: document.querySelectorAll('#settings-dialog input[name="reopen"]'),
  diagramColourRadios: document.querySelectorAll('#settings-dialog input[name="diagram-colours"]'),
  autoUpdate: document.getElementById("auto-update"),
  checkNow: document.getElementById("check-now"),
  updateStatus: document.getElementById("update-status"),
  settingsUpdate: document.getElementById("settings-update"),
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
  updateLater: document.getElementById("update-later"),
  emptyOpenBtn: document.getElementById("empty-open-btn"),
  emptyFolderBtn: document.getElementById("empty-folder-btn"),
  emptyReopenBtn: document.getElementById("empty-reopen-btn"),
  emptyReopenLabel: document.getElementById("empty-reopen-label"),
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
  toast: document.getElementById("toast"),
  copyIcon: document.getElementById("copy-section-icon"),
  diagramDialog: document.getElementById("diagram-dialog"),
  diagramView: document.getElementById("diagram-view"),
  diagramTools: document.getElementById("diagram-tools"),
  diagramLevel: document.getElementById("diagram-level"),
};

const state = {
  theme: null,
  /*
   * The light/dark side to prefer when moving between theme families. Kept
   * rather than read back off the current theme, because a family that has
   * only one side hands that one over — and re-deriving from it would make
   * stepping past Sakura silently change the side every later step lands on.
   * Only the toggle button changes it.
   */
  themeMode: "dark",
  themes: [],
  openMode: "tab",
  /**
   * Platform facts, filled in from `get_settings` at boot. The defaults are the
   * conservative reading — no cross-window drag, case-sensitive paths — so the
   * app behaves correctly for the brief moment before the answer arrives.
   */
  crossWindowDrag: false,
  caseInsensitivePaths: false,
  /**
   * The theme `applyTheme` falls back to when the chosen one will not load,
   * from `get_settings` at boot. Rust owns the name.
   */
  defaultTheme: null,
  /** The release `check_for_update` found, or null while there is none. */
  update: null,
  /** Root of the folder in the sidebar, or null while it is closed. */
  folder: null,
  /**
   * Whether the reader chose that folder. The tab showing it knows as much for
   * itself; this is for the window with no tab yet, whose first one inherits it.
   */
  folderPicked: false,
  /**
   * What the sidebar orders its files by, from `get_settings` at boot:
   * `"modified"` for newest first, `"name"` otherwise. Rust does the sorting;
   * this is only what gets handed to it.
   */
  folderSort: "name",
  /** Whose colours diagrams wear: `"theme"` or `"mermaid"`, from `get_settings` at boot. */
  diagramColours: "theme",
};

// What opens as a document, by kind. The Open dialog's filters are built
// from these too, so the two cannot drift apart. Rust's `MD_EXTS` also takes
// `.txt` from the command line and the sidebar; that is deliberately not
// offered here — see the note in `linux/t4-markdown-viewer.xml`.
const MD_EXTS = ["md", "markdown", "mdown", "mkd", "mdtext", "mdtxt", "mdwn", "mkdn"];
const JSON_EXTS = ["json", "jsonc"];
const DOC_LINK = new RegExp(`\\.(${[...MD_EXTS, ...JSON_EXTS].join("|")})$`, "i");
const IMG_LINK = /\.(svg|png|jpe?g|gif|webp|avif|bmp|ico)$/i;
const HAS_SCHEME = /^[a-z][a-z0-9+.-]*:/i;

/**
 * A link that names a file by drive and path: `C:\notes\spec.md`, `D:/a/b.md`.
 *
 * Not a leading slash. `/docs/guide.md` is how a repository's README names a
 * file from the repository's root, so `resolvePath` takes it from there — the
 * `repo` Rust found for the document — or, outside a repository, from the
 * document's folder. Reading it as the filesystem's root instead would break
 * those links to fix ones almost nobody writes.
 *
 * Not a network path either — `\\host\share\x.png`, `//host/x.png` — and on
 * purpose: a picture is fetched without a click, and a document able to name a
 * server could have Windows offer the reader's credentials to it.
 */
const ABSOLUTE_PATH = /^[a-z]:[\\/]/i;

/**
 * What a tab holds is read back off its path rather than stored beside it. That
 * keeps `makeTab`, `adoptTab` and the cross-window drag payload untouched: a
 * tab handed to another window arrives knowing what it is.
 */
function isImage(p) {
  return IMG_LINK.test(p);
}

/* ---------------- paths ---------------- */

/** `href` with its percent-escapes undone; malformed ones are used as written. */
function unescapeHref(href) {
  try {
    return decodeURIComponent(href);
  } catch {
    return href;
  }
}

/**
 * Resolve `rel` against `dir` — or on its own, if it is a full path — collapsing `.` and `..`. Forward-slash output.
 * A leading slash is taken from `repo`, the repository the document sits in, when it has one.
 */
function resolvePath(dir, rel, repo) {
  const decoded = unescapeHref(rel);
  // A full path stands on its own; only a relative one hangs off a folder.
  const absolute = ABSOLUTE_PATH.test(decoded);
  const base = repo && /^\/(?!\/)/.test(decoded) ? repo : dir;
  const joined = (absolute ? decoded : `${base}/${decoded}`).replace(/\\/g, "/");
  // A network path is `//server/share/...`: both leading slashes belong to the
  // root, and the server and share are part of it too — `..` climbing past them
  // would leave a path no longer pointing at any machine. Only the base can say
  // so; the joined string starts `//` for the POSIX root as well, since the
  // separator lands right behind its own leading slash.
  const unc = !absolute && base.replace(/\\/g, "/").startsWith("//");
  const floor = unc ? 4 : 1; // ["", "", server, share], or one leading segment
  const parts = joined.split("/");
  const out = [];
  for (let i = 0; i < parts.length; i++) {
    const p = parts[i];
    if (p === "" || p === ".") {
      if (i === 0 || (unc && i === 1)) out.push(p); // keep the leading empties for UNC-ish roots
      continue;
    }
    if (p === "..") {
      if (out.length > floor) out.pop();
      continue;
    }
    out.push(p);
  }
  return out.join("/");
}

/** Whether `href` names a file on this machine: relative to the document, or by its full path. */
function isLocal(href) {
  if (!href || href.startsWith("#")) return false;
  // Before the scheme test: `C:` reads as one.
  if (ABSOLUTE_PATH.test(unescapeHref(href))) return true;
  return !HAS_SCHEME.test(href) && !href.startsWith("//");
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

/** Whether `path` lies in `dir` or anywhere below it. */
function isInside(path, dir) {
  const root = `${normPath(dir).replace(/\/+$/, "")}/`;
  return normPath(path).startsWith(root);
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

/** Where Ctrl+click is a right-click and Cmd is the modifier that opens tabs. */
const isMac = navigator.userAgent.includes("Macintosh");

/** Guards against a slow load painting over a newer tab switch. */
let renderToken = 0;

/**
 * The render whose document is on screen. Behind `renderToken` from the moment
 * a switch begins until what it asked for is shown — and for that long the
 * page still belongs to the entry being left, not the active one.
 */
let shownToken = 0;

/**
 * The history entry whose document or picture is on screen, or null for the
 * empty and error panels — the one record of whose page that is, and so the
 * entry every reading position is banked on. It changes only as a page is
 * swapped in (or, for a jump between anchors, as the same page passes to the
 * next entry): while a switch or a refresh loads, it is still the entry whose
 * page the reader is scrolling. Settled (`shownToken === renderToken`), it is
 * the active entry or null. `show()` clears it whenever a panel replaces the
 * page, so no path can leave a collapsed page banking on an entry.
 */
let shownEntry = null;

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
    /** The sidebar's root for this tab, or null for the document's own folder. */
    folder: null,
    /** Whether the reader chose that folder, rather than the sidebar following to it. */
    picked: false,
    /** A new tab reads in whatever order is in force when it opens. */
    sort: state.folderSort,
    /** Folders open inside the tree. Tabs on one folder share its live tree, and so this. */
    expanded: [],
    /** What the filter box held for this tab. Not saved: a search, not a setting. */
    filter: "",
    entries: [{ path, scrollY: 0 }],
    index: 0,
  };
}

/** Snapshot the reading position so Back and tab switches return to the spot. */
function rememberScroll() {
  // On the entry whose page is on screen, not the active one: while a switch
  // loads, the active entry has moved on and the page being scrolled is still
  // the one being left, which keeps what the reader does to it. The empty and
  // error panels have no entry — the error panel collapses the page, and the
  // clamp's scroll would bank 0 over the entry's real spot.
  if (!shownEntry) return;
  // A picture scrolls inside its own box, and in two directions; it also has a
  // zoom to keep.
  if (isImage(shownEntry.path)) rememberImage();
  else shownEntry.scrollY = window.scrollY;
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
function resolveMedia(root, dir, repo) {
  root.querySelectorAll("img[src], video[src], audio[src], source[src]").forEach((el) => {
    const raw = el.getAttribute("src");
    if (!isLocal(raw)) return;
    const file = resolvePath(dir, raw, repo);
    // A picture nothing was watching may have changed unseen, so the webview's
    // cached copy cannot be trusted; a bump refetches it. A watched one is left
    // alone — that is what keeps a refresh's scroll restore honest, since the
    // page then has its pictures' full height straight away.
    if (isImage(file) && !isWatched(file)) bumpAsset(file);
    el.setAttribute("src", assetUrl(file));
    // Remember the file behind the picture so a click can open it full size —
    // a diagram at column width is often too small to read. Images only: the
    // same loop also rewrites video and audio, which have their own controls.
    if (el.tagName === "IMG" && isImage(file)) {
      el.dataset.file = file;
      // Reached by Tab and opened by Enter or Space too — unless it is a
      // link's picture, where the link is what a key press means.
      if (!el.closest("a")) {
        el.tabIndex = 0;
        el.setAttribute("role", "button");
        if (!el.alt) el.setAttribute("aria-label", baseName(file));
      }
    }
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

/* ---------------- diagrams ---------------- */

/**
 * The diagram renderer: mermaid, in a hidden frame of its own. It is 3.5 MB of
 * script, so it loads the first time a document has a diagram. And it runs in
 * a frame sandboxed to scripts alone, with no origin: mermaid builds each
 * drawing live in its document before handing back the string, so the
 * unsanitised drawing is never in the app's page, and nothing it runs can
 * reach the page or Tauri. The frame must be laid out, not hidden, for mermaid
 * to measure text.
 */
let rendererLoad = null;
function loadRenderer() {
  if (rendererLoad) return rendererLoad;
  const load = new Promise((resolve, reject) => {
    const frame = document.createElement("iframe");
    frame.setAttribute("sandbox", "allow-scripts");
    frame.setAttribute("aria-hidden", "true");
    // Never focused or clicked into; its scripts still run.
    frame.inert = true;
    frame.style.cssText = "position:fixed;left:-20000px;top:0;width:1200px;height:1200px;border:0";
    frame.srcdoc =
      '<!doctype html><html><head><script src="vendor/mermaid.min.js"></script>' +
      '<script src="diagram-frame.js"></script></head><body style="margin:0"></body></html>';
    let n = 0;
    // ~5× the slowest real diagram measured (a 600-node mindmap, ~6 s under
    // load).
    const RENDER_TIMEOUT = 30000;
    let settled = false;
    const waiting = new Map();
    const fail = () => {
      if (settled) return;
      settled = true;
      removeEventListener("message", onMessage);
      frame.remove();
      if (rendererLoad === load) rendererLoad = null; // the next document tries again
      reject(new Error("The diagram renderer did not load."));
    };
    // A frame that stopped answering — lost, or stuck in a render — would
    // hold the queue, and with it every diagram document after this one. Give
    // up on what it owes and drop it: whatever is drawn next gets a fresh
    // frame. Where the frame runs in the page's own process — as in WebView2
    // (measured) and likely WebKit — a render stuck in a loop freezes the page
    // too, and nothing here gets to run; what this rescues is a frame that
    // stops answering.
    const stop = () => {
      removeEventListener("message", onMessage);
      frame.remove();
      if (rendererLoad === load) rendererLoad = null;
      for (const settle of waiting.values()) settle({ error: "The diagram took too long to draw." });
      waiting.clear();
    };
    setTimeout(fail, 15000);
    const render = (config, id, source) => {
      // Given up on, or taken away: what is still to draw goes to a fresh frame.
      if (!frame.isConnected) {
        stop();
        return loadRenderer().then((next) => next(config, id, source));
      }
      return new Promise((settle) => {
        const mine = ++n;
        const timer = setTimeout(stop, RENDER_TIMEOUT);
        waiting.set(mine, (result) => (clearTimeout(timer), settle(result)));
        // The width goes with the request: the frame may run in a process of
        // its own, where a resize could land after the render.
        frame.contentWindow.postMessage({ n: mine, config, id, source, width: document.body.offsetWidth }, "*");
      });
    };
    function onMessage(e) {
      if (e.source !== frame.contentWindow || typeof e.data !== "object" || !e.data) return;
      if (e.data.failed) return fail();
      if (e.data.ready) return settled || ((settled = true), resolve(render));
      const settle = waiting.get(e.data.n);
      if (!settle) return;
      waiting.delete(e.data.n);
      // Strings only: the frame runs whatever the file hands mermaid.
      const { svg, diagramType, error } = e.data;
      // Pairs of strings only, for the same reason.
      const links = Array.isArray(e.data.links)
        ? e.data.links.filter((p) => Array.isArray(p) && typeof p[0] === "string" && typeof p[1] === "string")
        : [];
      settle(typeof svg === "string" ? { svg, diagramType: String(diagramType), links } : { error: String(error ?? "The diagram did not draw.") });
    }
    addEventListener("message", onMessage);
    document.body.append(frame);
    // The frame's scripts have run by its load event; a bridge that never
    // loaded — or a webview that refused it — says nothing, so give it a
    // moment past that rather than holding the document up for the backstop.
    // Listened for only once the frame is in: a webview that fires `load` for
    // the frame's initial blank document as it is inserted must not start the
    // grace before mermaid has even loaded, and the real srcdoc load is always
    // later.
    frame.addEventListener("load", () => setTimeout(fail, 1000));
  });
  rendererLoad = load;
  return load;
}

const MERMAID_BLOCK = "pre > code.language-mermaid";

/**
 * Finished diagrams, `{ svg }` or `{ error }`, by look and source. Filled
 * before a document is painted, so `renderDiagrams` swaps them in without
 * waiting — which is what lets the scroll restore land on a page that already
 * has its full height. A save that leaves a diagram alone redraws it for free.
 * Kept in the order last used, oldest first, so what goes when it is full is
 * what the window has shown least recently.
 */
const diagrams = new Map();
const DIAGRAMS_KEPT = 200;
let diagramId = 0;
/** One warm at a time: the frame renders one diagram at a time, and a warm's look must not change under it. */
let diagramQueue = Promise.resolve();
/**
 * Diagram types `warmSources` draws despite foreignObjects in their drawing:
 * journey and venn write only plain text into theirs (measured on mermaid
 * 11.17.2 — markup from the file arrives escaped). Not architecture: a
 * service's text icon goes in as filtered HTML. Any mermaid bump re-measures
 * this list.
 */
const TEXT_ONLY_FO = new Set(["journey", "venn"]);

/**
 * Whether every rule in a drawing's styles is scoped under its root, as
 * mermaid means them to be. mermaid scopes them itself, but a stray `"` in a
 * diagram's own theme values makes its CSS compiler and the browser disagree
 * about where a string ends, and the rest of its stylesheet then applies to
 * the whole app. Parsed the way the page will parse it. Keyframes are global
 * by nature, and harmless: the app has none of its own.
 */
function stylesStayInside(root) {
  const scope = `#${root.id}`;
  // Split on top-level commas only: `:is(a, b)` is one selector. A comma or
  // bracket that is escaped (`.a\(`) or inside a string (`[title="("]`) is
  // part of a name, so it neither splits nor nests — otherwise a selector
  // could hide a second, unscoped one behind it. A list whose brackets do not
  // balance is not trusted at all (null).
  const selectors = (text) => {
    const out = [""];
    let depth = 0;
    let quote = null;
    for (let i = 0; i < text.length; i++) {
      let c = text[i];
      if (c === "\\") c += text[++i] ?? "";
      else if (quote) {
        if (c === quote) quote = null;
      } else if (c === '"' || c === "'") quote = c;
      else if (c === "(" || c === "[") depth++;
      else if ((c === ")" || c === "]") && --depth < 0) return null;
      else if (c === "," && !depth) {
        out.push("");
        continue;
      }
      out[out.length - 1] += c;
    }
    return depth || quote ? null : out.map((s) => s.trim());
  };
  // The root itself, or inside it — not its siblings (`~`, `+`), and not
  // `#Mermaid-10` for `#Mermaid-1`.
  const scoped = (s) => s.startsWith(scope) && /^(\s*>|\s+[^\s~+]|$)/.test(s.slice(scope.length));
  const ok = (rules) =>
    [...rules].every((r) =>
      r instanceof CSSKeyframesRule ||
      // A nested rule only comes from the escape: mermaid's compiler flattens.
      (r instanceof CSSStyleRule ? !!selectors(r.selectorText)?.every(scoped) && !r.cssRules?.length
        : r instanceof CSSGroupingRule && ok(r.cssRules)));
  // Parsed the way the page will parse it, by a `<style>` that matches no
  // media, so it applies to nothing while it is read.
  const probe = document.createElement("style");
  probe.media = "not all";
  try {
    return [...root.querySelectorAll("style")].every((style) => {
      probe.textContent = style.textContent;
      document.head.append(probe);
      return ok(probe.sheet.cssRules);
    });
  } finally {
    probe.remove();
  }
}

/**
 * One pixel to paint colours on and read back. A computed colour can be any
 * CSS form — `oklch(…)`, `color(srgb 0.97 …)` — whose numbers are not 0–255
 * channels, and mermaid takes none of those; a canvas hands back sRGB bytes
 * for all.
 */
let pixel = null;

/**
 * Hidden stand-ins for prose, code and a link — a `<p>`, a `<pre>` and a `<p>`
 * holding an `<a>` — put straight into the document for each read, so the
 * theme styles them like its own.
 */
let themeProbe = null;

/**
 * What a drawing depends on: the side of the page actually on screen — read
 * off its background, since a theme that failed to load leaves another's
 * there — and the font. Not the theme's name: editing a theme keeps it. With
 * the theme's colours, also the palette: the page's text, code background and
 * link colour, as the theme styles a document.
 */
function diagramLook() {
  pixel ??= document.createElement("canvas").getContext("2d", { willReadFrequently: true });
  // A colour as sRGB bytes, painted over `under` — any CSS form, see-through or not.
  const bytes = (css, under) => {
    pixel.fillStyle = under;
    pixel.fillRect(0, 0, 1, 1);
    pixel.fillStyle = css;
    pixel.fillRect(0, 0, 1, 1);
    return [...pixel.getImageData(0, 0, 1, 1).data.slice(0, 3)];
  };
  // On the page's own canvas colour first: that is what the webview paints
  // behind a page with no background — dark under a dark `color-scheme` — and
  // a see-through colour read on its own comes back black. The overlay's
  // background colour is exactly that system `Canvas`.
  const bg = bytes(getComputedStyle(document.body).backgroundColor, getComputedStyle(els.diagramDialog).backgroundColor);
  const dark = 0.299 * bg[0] + 0.587 * bg[1] + 0.114 * bg[2] < 128;
  const fontFamily = getComputedStyle(els.content).fontFamily;
  if (state.diagramColours !== "theme") return { dark, fontFamily, palette: null, key: `m\0${dark}\0${fontFamily}` };
  if (!themeProbe) {
    const t = document.createElement("template");
    t.innerHTML = '<p>x</p><pre><code>x</code></pre><p><a href="#">x</a></p>';
    themeProbe = [...t.content.children];
    for (const el of themeProbe) {
      el.setAttribute("aria-hidden", "true");
      el.style.cssText = "position:absolute;left:-10000px;top:0;visibility:hidden";
    }
  }
  const hexOf = (c) => `#${c.map((v) => v.toString(16).padStart(2, "0")).join("")}`;
  const page = hexOf(bg);
  const over = (el, prop) => bytes(getComputedStyle(el)[prop], page);
  // Straight into the document itself, for this read only, so a theme's
  // `#content p` or `#content > p` colours them as well as its
  // `.markdown-body p`; out again before anything else can run and find them
  // there.
  els.content.append(...themeProbe);
  let fg, code, accent;
  try {
    fg = over(themeProbe[0], "color");
    code = over(themeProbe[1], "backgroundColor");
    accent = over(themeProbe[2].querySelector("a"), "color");
  } finally {
    for (const el of themeProbe) el.remove();
  }
  const mix = (a, b, t) => hexOf(a.map((v, i) => Math.round(v * t + b[i] * (1 - t))));
  const lum = (c) =>
    c.map((v) => ((v /= 255) <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4)).reduce((s, v, i) => s + v * [0.2126, 0.7152, 0.0722][i], 0);
  const contrast = (a, b) => (Math.max(lum(a), lum(b)) + 0.05) / (Math.min(lum(a), lum(b)) + 0.05);
  const hue = ([r, g, b]) => {
    const max = Math.max(r, g, b), d = max - Math.min(r, g, b);
    if (!d) return 210;
    const h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
    return Math.round(h * 60);
  };
  // Categories — pie slices, git branches, mindmap and timeline sections,
  // event-modeling boxes, chart bars — need colours that differ: hues stepped
  // by the golden angle from the link colour's. Each is taken toward the page's
  // side until the text reads on it at 4.5:1 — or 3:1 on a theme whose own
  // text is under 5:1 on its page (Solarized's is 4.1–4.7), where a higher
  // bar would leave the colours, and the lines mermaid draws in them, barely
  // apart from the page.
  const target = contrast(fg, bg) >= 5 ? 4.5 : 3;
  const hueAt = (i) => (hue(accent) + i * 137.5) % 360;
  const cats = Array.from({ length: 12 }, (_, i) => {
    for (let l = dark ? 32 : 82; ; l += dark ? -2 : 2) {
      const c = bytes(`hsl(${hueAt(i)} ${dark ? 40 : 60}% ${l}%)`, page);
      if (contrast(fg, c) >= target || l <= 0 || l >= 100) return hexOf(c);
    }
  });
  // Venn writes a set's label in the set's own colour, 30 lighter or darker,
  // so its sets sit mid-way rather than behind the text.
  const venn = Array.from({ length: 8 }, (_, i) => hexOf(bytes(`hsl(${hueAt(i)} 60% ${dark ? 60 : 45}%)`, page)));
  const each = (prefix, colours, from = 0) => Object.fromEntries(colours.map((c, i) => [`${prefix}${i + from}`, c]));
  // Measured across the bundled themes (2026-09-27): their own border colours
  // are too faint to outline a node, so borders and lines are the text mixed
  // into the page; a code background as faint as the page gets the same.
  const surface = contrast(code, bg) >= 1.05 ? hexOf(code) : mix(fg, bg, 0.08);
  const border = mix(fg, bg, 0.6);
  const text = hexOf(fg);
  const link = hexOf(accent);
  const faint = mix(fg, bg, 0.04);
  const palette = {
    // Inside the variables: mermaid reads the side from here, not the top level.
    darkMode: dark, background: page, fontFamily,
    primaryColor: surface, mainBkg: surface, primaryTextColor: text, textColor: text, titleColor: text,
    primaryBorderColor: border, nodeBorder: border, clusterBorder: border,
    secondaryColor: mix(accent, bg, 0.18), secondaryTextColor: text, secondaryBorderColor: border,
    tertiaryColor: faint, tertiaryTextColor: text, tertiaryBorderColor: border,
    clusterBkg: faint, lineColor: mix(fg, bg, 0.75), edgeLabelBackground: page,
    noteBkgColor: mix(accent, bg, 0.14), noteTextColor: text, noteBorderColor: mix(accent, bg, 0.6),
    ...each("pie", cats, 1), pieSectionTextColor: text, pieTitleTextColor: text, pieLegendTextColor: text,
    pieStrokeColor: border, pieOuterStrokeColor: border,
    ...each("git", cats.slice(0, 8)), ...each("gitBranchLabel", Array(8).fill(text)),
    ...each("cScale", cats), ...each("cScaleLabel", Array(12).fill(text)),
    ...each("fillType", cats.slice(0, 8)), ...each("venn", venn, 1),
    // What mermaid's base theme fixes at light values whatever the side.
    doneTaskBkgColor: mix(fg, bg, 0.2), doneTaskBorderColor: border, altSectionBkgColor: faint,
    excludeBkgColor: mix(fg, bg, 0.08), taskTextClickableColor: link, vertLineColor: link,
    archGroupBorderColor: border, faceColor: surface,
    emUiFill: surface, emSwimlaneBackgroundOdd: faint, emSwimlaneBackgroundStroke: border, emUiStroke: border,
    emProcessorFill: cats[0], emReadModelFill: cats[1], emCommandFill: cats[2], emEventFill: cats[3],
    emProcessorStroke: border, emReadModelStroke: border, emCommandStroke: border, emEventStroke: border,
    packet: { startByteColor: text, endByteColor: text, labelColor: text, titleColor: text, blockStrokeColor: border, blockFillColor: surface },
    // Whole, not just the palette: mermaid puts a nested object given here in
    // place of the one it derives, rather than merging the two.
    xyChart: {
      backgroundColor: page, titleColor: text, dataLabelColor: text, legendTextColor: text,
      xAxisTitleColor: text, xAxisLabelColor: text, xAxisTickColor: text, xAxisLineColor: text,
      yAxisTitleColor: text, yAxisLabelColor: text, yAxisTickColor: text, yAxisLineColor: text,
      plotColorPalette: cats.slice(0, 10).join(","),
    },
    // Whole for the same reason, with mermaid's own sizes but not its rings:
    // those are a fixed light grey, nested and each filled, so its 0.3 stacks
    // to a solid band on either side that the curve drowns in.
    radar: {
      axisColor: mix(fg, bg, 0.75), axisStrokeWidth: 2, axisLabelFontSize: 12, curveOpacity: 0.5, curveStrokeWidth: 2,
      graticuleColor: border, graticuleStrokeWidth: 1, graticuleOpacity: 0.06, legendBoxSize: 12, legendFontSize: 12,
    },
  };
  return { dark, fontFamily, palette, key: `t\0${JSON.stringify(palette)}` };
}

function diagramKey(look, source) {
  return `${look.key}\0${source}`;
}

/**
 * Rename ids inside a drawing, and every reference to them: `url(#…)` in any
 * attribute but a human-readable one (`title`, `data-*`, `aria-*` other than
 * the id lists), `href="#…"` on anything but a link — a diagram's `click` link
 * names a place in the document, not a part of the drawing — the aria id
 * lists, and selectors and `url(#…)` in its own `<style>`. Text is left
 * alone, so a label that mentions an id reads the same; so are declarations,
 * where `#aaa` is a colour.
 */
function renameIds(svg, rename) {
  const renamed = new Map();
  for (const el of [svg, ...svg.querySelectorAll("[id]")]) {
    if (!el.id) continue;
    const to = rename(el.id);
    if (to && to !== el.id) renamed.set(el.id, to), (el.id = to);
  }
  if (!renamed.size) return;
  const url = /url\((['"]?)#([^'")]+)\1\)/g;
  const toUrl = (m, q, id) => (renamed.has(id) ? `url(${q}#${renamed.get(id)}${q})` : m);
  for (const el of [svg, ...svg.querySelectorAll("*")]) {
    for (const attr of [...el.attributes]) {
      // Words, not references: a label or title that mentions `url(#…)` reads
      // the same. The two aria id lists are references, and are renamed below.
      const words =
        attr.localName === "title" ||
        attr.name.startsWith("data-") ||
        (attr.name.startsWith("aria-") && attr.name !== "aria-labelledby" && attr.name !== "aria-describedby");
      if (words) continue;
      let value = attr.value.replace(url, toUrl);
      if (attr.localName === "href" && el.localName !== "a" && value.startsWith("#") && renamed.has(value.slice(1)))
        value = `#${renamed.get(value.slice(1))}`;
      if (attr.name === "aria-labelledby" || attr.name === "aria-describedby")
        value = value.split(/\s+/).map((id) => renamed.get(id) ?? id).join(" ");
      if (value !== attr.value) attr.value = value;
    }
  }
  const hash = (m, id) => (renamed.has(id) ? `#${renamed.get(id)}` : m);
  for (const style of svg.querySelectorAll("style"))
    style.textContent = style.textContent
      .replace(/[^{}]+(?=\{)/g, (selector) => selector.replace(/#([\w-]+)/g, hash))
      .replace(url, toUrl);
}

/** `warmSources` for the diagrams in a document's html. */
function warmDiagrams(html, stale) {
  // Both renderers escape `"` in text, so only a real code block matches.
  if (!html.includes('class="language-mermaid"')) return Promise.resolve();
  const t = document.createElement("template");
  t.innerHTML = html;
  return warmSources(new Set([...t.content.querySelectorAll(MERMAID_BLOCK)].map((c) => c.textContent)), stale);
}

/**
 * Draw every source not already in `diagrams`. Resolves once each has drawn or
 * failed, or as soon as `stale()` says a newer switch has won — the queue is
 * one at a time, and a document left behind must not hold up the one that
 * replaced it. Never rejects: a renderer that will not load leaves the blocks
 * as code.
 */
function warmSources(sources, stale) {
  const run = async () => {
    if (stale()) return;
    // Read before any wait: the theme can change while this is out, and a
    // diagram drawn in one look must not be filed under the next.
    const look = diagramLook();
    const todo = [...sources].filter((s) => !diagrams.has(diagramKey(look, s)));
    // This document's drawings become the newest, so the oldest are other
    // documents'; drop those to make room, never this one's — a document with
    // more diagrams than the limit keeps them all.
    const mine = new Set();
    for (const s of sources) {
      const key = diagramKey(look, s);
      mine.add(key);
      const done = diagrams.get(key);
      if (done) diagrams.delete(key), diagrams.set(key, done);
    }
    for (const key of diagrams.keys()) {
      if (diagrams.size + todo.length <= DIAGRAMS_KEPT || mine.has(key)) break;
      diagrams.delete(key);
    }
    if (!todo.length) return;
    const config = {
      startOnLoad: false,
      securityLevel: "strict",
      suppressErrorRendering: true,
      // Labels as SVG text, not HTML: strict mode still lets a filtered subset
      // of markup through, and nothing from the file reaches the page as HTML.
      // Flowcharts read their own, deprecated, copy of the switch.
      htmlLabels: false,
      flowchart: { htmlLabels: false },
      // mermaid's own six, plus the two switches above: a diagram's directive
      // or front matter could otherwise turn HTML labels back on. The cost is
      // that a diagram can no longer set its own flowchart options. Plus, with
      // the theme's colours, the five that would re-colour a diagram wholesale.
      secure: [
        "secure",
        "securityLevel",
        "startOnLoad",
        "maxTextSize",
        "suppressErrorRendering",
        "maxEdges",
        "htmlLabels",
        "flowchart",
        ...(look.palette ? ["theme", "themeVariables", "darkMode", "fontFamily", "themeCSS"] : []),
      ],
      fontFamily: look.fontFamily,
      // The theme's colours, or mermaid's own light or dark. With the theme's,
      // a diagram cannot re-colour itself wholesale — its own `theme`,
      // `themeVariables` and the like are locked above — though `classDef`,
      // `style` and a type's own colour options still colour it; with
      // mermaid's, it may, as on GitHub.
      ...(look.palette ? { theme: "base", themeVariables: look.palette } : { theme: look.dark ? "dark" : "default" }),
    };
    for (const source of todo) {
      // Let a click that switches away run between diagrams, rather than after the last.
      await new Promise((r) => setTimeout(r));
      if (stale()) return; // the rest wait for this document to come back
      // Each time, not once: after a frame is given up on, the next diagram
      // needs the fresh one — and a renderer that will not load stops the pass
      // here, caching nothing, as it always has.
      const render = await loadRenderer();
      let done;
      try {
        // mermaid draws any label with `$$…$$` in it as HTML, whatever
        // `htmlLabels` and `secure` say, sanitised only of `<style>`: its
        // markup, style attributes included, would reach the page. Its own
        // test, so what it would see as math is refused here.
        if (/\$\$(.*?)\$\$/.test(source))
          throw new Error("Math in labels ($$…$$) isn't supported here: this viewer draws diagram labels as plain text.");
        // Capitalised: comrak's heading slugs are lower case, so this id can
        // never collide with a `#section` link target.
        const id = `Mermaid-${++diagramId}`;
        const drawn = await render(config, id, source);
        if (typeof drawn.svg !== "string") throw new Error(drawn.error || "The diagram did not draw.");
        const { svg, diagramType } = drawn;
        // Any other way mermaid turns HTML labels back on is closed the same
        // way, except for `TEXT_ONLY_FO`. And event modeling: mermaid always
        // writes its entity names, and their data, as HTML (`<b>${…}</b>`
        // after `sanitizeText`), no setting changes that, and the owner
        // accepted it for this one type.
        if (diagramType !== "eventmodeling" && !TEXT_ONLY_FO.has(diagramType) && /<foreignObject/i.test(svg))
          throw new Error("This diagram needs HTML labels, which this viewer does not draw.");
        const t = document.createElement("template");
        t.innerHTML = svg;
        const root = t.content.querySelector("svg");
        // The style check trusts the root's id as the scope every rule must sit
        // under, so it must be the id the page asked for, not whatever the
        // frame sends back: a root named `content` would scope rules to the page.
        if (root?.id !== id) throw new Error("The diagram did not draw.");
        if (!stylesStayInside(root))
          throw new Error("This diagram's styles reach outside it, so it isn't drawn.");
        // Every id goes under the drawing's own, so a group named like part of
        // the app — `icons`, `tree` — can neither wear the app's styles nor be
        // what a `#tree` link finds.
        renameIds(root, (old) => (old.startsWith(root.id) ? null : `${root.id}-${old}`));
        // A task with a link becomes one: its bar and its label each wrapped in
        // the kind of link a flowchart's `click` makes, which the page already
        // follows. Not `about:blank`: that is mermaid's mark for a URL it
        // would not trust, and there is nothing to follow.
        for (const [task, url] of drawn.links ?? []) {
          if (url === "about:blank") continue;
          for (const suffix of ["", "-text"]) {
            // By element too: a task named `t1-text` has a bar with the id of
            // `t1`'s label.
            const el = root.querySelector(`${suffix ? "text" : "rect"}#${CSS.escape(`${root.id}-${task}${suffix}`)}`);
            if (!el) continue;
            const a = document.createElementNS("http://www.w3.org/2000/svg", "a");
            a.setAttributeNS("http://www.w3.org/1999/xlink", "xlink:href", url);
            el.replaceWith(a);
            a.append(el);
          }
        }
        // A task mermaid marked as a link but that got none — an unsafe URL,
        // or a `call` — should not look like one.
        if (diagramType === "gantt")
          for (const el of root.querySelectorAll(".clickable")) if (!el.closest("a")) el.classList.remove("clickable");
        // The root alone: the check above read only its styles, so a `<style>`
        // or anything else sent beside the drawing must never reach the page.
        done = { svg: root.outerHTML };
      } catch (err) {
        done = { error: String(err?.message ?? err) };
      }
      diagrams.set(diagramKey(look, source), done);
    }
    // The look changed while these drew, so draw them as they will be shown.
    if (!stale() && diagramLook().key !== look.key) return run();
  };
  diagramQueue = diagramQueue.then(run).catch(console.error);
  return diagramQueue;
}

/**
 * Each drawn diagram's figure, to `{ pre, look }`: the code block it replaced —
 * comrak's own, with its attributes — whose text is the diagram's source, and
 * the look key it was last drawn in. For `redrawDiagrams` to put back, and the
 * overlay to know which drawing it is a copy of.
 */
const diagramBlocks = new WeakMap();

/*
 * Only our own figures are figures. A diagram can wear the class itself —
 * `classDef mermaid-diagram …` puts it on its nodes — and such a node has no
 * block to go back to or source to show.
 */

/** The figure of ours `el` is inside, or null. */
function figureOf(el) {
  for (let f = el.closest(".mermaid-diagram"); f; f = f.parentElement?.closest(".mermaid-diagram"))
    if (diagramBlocks.has(f)) return f;
  return null;
}

/** What a key press can open: a diagram, or a picture outside a link. */
const OPENABLE = "div.mermaid-diagram[tabindex], img[data-file][tabindex]";

/**
 * A picture opened from the keyboard, in a tab of its own: the document it was
 * on and where it stood there, so that closing the tab puts focus back on it.
 */
let pictureBack = null;

/** What makes an openable element the same one after a re-render: its source or its file. */
function openableKey(el) {
  return diagramBlocks.get(el)?.pre.textContent ?? el.dataset.file;
}

/** Where an openable element stands among the page's, and what it is. */
function markOpenable(el) {
  return { at: [...els.content.querySelectorAll(OPENABLE)].indexOf(el), what: openableKey(el) };
}

/**
 * Focus the element `mark` recorded, found again by its place among the
 * page's — a render makes new ones — and only if it is still the same one
 * there, or focus would land on a neighbour.
 */
function refocusOpenable(mark) {
  const again = els.content.querySelectorAll(OPENABLE)[mark.at];
  if (again && openableKey(again) === mark.what) again.focus({ preventScroll: true });
}

/*
 * A sequence diagram's `link`/`links` give an actor a menu, which mermaid
 * draws hidden and opens with a handler the app's strict mode strips. So the
 * page opens it: a click on the actor's box or lifeline shows it; a click
 * anywhere else in the document, or Escape, puts it away.
 */

/** The link menu of the actor whose box or lifeline `el` is in, within `fig`, or null. */
function actorMenu(fig, el) {
  // The top box's group and every lifeline name their actor; a bottom box
  // only on its rect, which a database or queue, drawn in paths, has not.
  const name =
    el.closest('g[data-et="participant"], line[data-et="life-line"]')?.getAttribute("data-id") ??
    el.closest("g")?.querySelector("rect.actor[name]")?.getAttribute("name");
  if (!name) return null;
  // By name, not number: mermaid numbers actors from a count that runs on
  // across diagrams.
  const line = [...fig.querySelectorAll("line.actor-line[name]")].find((l) => l.getAttribute("name") === name);
  return line ? fig.querySelector(`#${CSS.escape(`${line.id}_popup`)}`) : null;
}

/** Put away every open actor menu but `except`; whether one was open. */
function closeActorMenus(except) {
  let closed = false;
  for (const menu of els.content.querySelectorAll('g.actorPopupMenu[id$="_popup"][display="block"]'))
    if (menu !== except) menu.setAttribute("display", "none"), menu.removeAttribute("transform"), (closed = true);
  return closed;
}

/** The figures on the page. */
function diagramFigures() {
  return [...els.content.querySelectorAll(".mermaid-diagram")].filter((f) => diagramBlocks.has(f));
}

/** A failed diagram's block stays as code, with the reason under it. */
function sayDiagramFailed(pre, error) {
  const p = document.createElement("p");
  p.className = "mermaid-error";
  p.textContent = error;
  pre.after(p);
}

/**
 * Swap each Mermaid block for its warmed diagram. One that failed keeps its
 * source and says why. Drawn in `look`, or the current one if not given.
 */
function renderDiagrams(root, look) {
  // The look costs a style recalc; a document without a diagram needs none.
  if (!root.querySelector(MERMAID_BLOCK)) return;
  look ??= diagramLook();
  root.querySelectorAll(MERMAID_BLOCK).forEach((code) => {
    const done = diagrams.get(diagramKey(look, code.textContent));
    const pre = code.parentElement;
    if (done?.svg) {
      const fig = document.createElement("div");
      fig.className = "mermaid-diagram";
      // Reached by Tab and opened by Enter or Space (`onKeydown`). A group
      // rather than a button: a button's content is presentational, which
      // would hide the drawing's own links from a screen reader.
      fig.tabIndex = 0;
      fig.setAttribute("role", "group");
      fig.setAttribute("aria-label", "Diagram — Enter opens it full window");
      fig.innerHTML = done.svg;
      diagramBlocks.set(fig, { pre, look: look.key });
      pre.replaceWith(fig);
    } else if (done?.error) {
      sayDiagramFailed(pre, done.error);
    }
  });
  numberRepeats(root);
}

/**
 * The same diagram twice in a document is the same drawing twice, ids and
 * all. Each copy after the first gets its own, numbered under the first's —
 * `Mermaid-3x2`, `Mermaid-3x3`; mermaid never puts an `x` straight after the
 * root, so no id of its own can match — and every reference inside a copy
 * then finds that copy, on engines that do not paint a reference into a
 * hidden twin.
 */
function numberRepeats(root) {
  const seen = new Map();
  for (const fig of root.querySelectorAll(".mermaid-diagram")) {
    // Only our own figures: a diagram's nodes can wear the class too.
    if (!diagramBlocks.has(fig)) continue;
    const svg = fig.querySelector(":scope > svg");
    if (!svg) continue;
    // Counted by the first copy's id, whether this copy is fresh from the
    // cache or already numbered, so a copy re-set on its own gets its own
    // number back rather than a sibling's.
    const id = svg.id;
    const base = id.replace(/x\d+$/, "");
    const k = (seen.get(base) ?? 0) + 1;
    seen.set(base, k);
    const want = k > 1 ? `${base}x${k}` : base;
    if (id !== want) renameIds(svg, (old) => (old.startsWith(id) ? `${want}${old.slice(id.length)}` : null));
  }
}

/**
 * Draw the document's diagrams again in the current look, from the sources on
 * the page rather than the file: re-reading it would put the error panel up
 * for a file that has since gone, and lose `:target`.
 */
async function redrawDiagrams() {
  // Mid-switch the page on screen is on its way out, and the one coming in
  // reads the look when it draws.
  if (shownToken !== renderToken) return;
  const token = renderToken;
  const stale = () => token !== renderToken; // every paint is a new render
  const figs = diagramFigures();
  const blocks = [...els.content.querySelectorAll(MERMAID_BLOCK)];
  const sources = new Set([
    ...figs.map((f) => diagramBlocks.get(f).pre.textContent),
    ...blocks.map((c) => c.textContent),
  ]);
  await warmSources(sources, stale);
  if (stale()) return;
  // Blocks that had failed are drawn afresh, before any figure below turns
  // back into one; their old reasons go first.
  // `p.`: a diagram can give its own nodes the class too.
  els.content.querySelectorAll("p.mermaid-error").forEach((p) => p.remove());
  const look = diagramLook();
  renderDiagrams(els.content, look);
  // A figure keeps its node — and with it its place and the scroll anchor —
  // and only its drawing changes. One that fails in this look goes back to
  // its block, highlighted as `renderDocument` would have.
  for (const fig of figs) {
    // An overlapping redraw that landed first already put it back as a block.
    if (!fig.isConnected) continue;
    const drawn = diagramBlocks.get(fig);
    // Or drew it in this look already: parsing the same SVG again is waste.
    if (drawn.look === look.key) continue;
    const done = diagrams.get(diagramKey(look, drawn.pre.textContent));
    if (done?.svg) {
      fig.innerHTML = done.svg;
      drawn.look = look.key;
      continue;
    }
    fig.replaceWith(drawn.pre);
    if (done?.error) sayDiagramFailed(drawn.pre, done.error);
    if (!drawn.pre.querySelector(".hljs")) highlight(drawn.pre);
  }
  numberRepeats(els.content);
  // An overlay opened while this warmed is a copy of the old look's drawing.
  closeStaleDiagram();
}

/* ---------------- diagram overlay ---------------- */

/*
 * A diagram at column width can be too small to read, and unlike a picture it
 * has no file for the image viewer to open in a tab. A click opens a copy of it
 * full window instead, with the viewer's zoom and pan; Escape comes back to the
 * same spot. 100% is what fits, as for an SVG in the viewer.
 */
let zoomed = null; // { svg, ratio, scale, source, look, path, back } while the overlay is open
const diagramPan = dragPan(els.diagramView, () => zoomed);

/** Measured against the panel rather than the scroll box, for the reason `fitWidth` gives. */
function diagramFit() {
  const box = els.diagramDialog;
  return Math.max(1, Math.min(box.clientWidth, box.clientHeight * zoomed.ratio));
}

/** Zoom the overlay's copy about a point (`zoomAbout`). */
function zoomDiagram(scale, clientX, clientY) {
  if (!zoomed) return;
  zoomAbout(
    els.diagramView,
    zoomed.svg,
    () => {
      zoomed.scale = Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, scale));
      const width = diagramFit() * zoomed.scale;
      zoomed.svg.style.width = `${width}px`;
      zoomed.svg.style.height = `${width / zoomed.ratio}px`;
      els.diagramLevel.textContent = `${Math.round(zoomed.scale * 100)}%`;
    },
    clientX,
    clientY,
  );
}

function openDiagram(fig) {
  const { pre, look } = diagramBlocks.get(fig);
  // Not in a look the page no longer has — its side or font changed, and a
  // redraw is on its way or, under a theme whose colours move without a theme
  // change or an OS switch (a transition, a media query on the width), not
  // coming at all. The copy would be the old look's on the new look's backdrop,
  // so bring the drawing up to date instead; a later click or Enter opens it.
  // With a redraw already under way this one has nothing left to draw.
  if (look !== diagramLook().key) return redrawDiagrams().catch(console.error);
  const original = fig.querySelector("svg");
  // Its shape as drawn on the page. Not off the viewBox, which WebKit hands
  // back as null when a diagram has none.
  const { width, height } = original.getBoundingClientRect();
  // Its own ids: the copy's styles and arrowheads then point at itself rather
  // than at the original, which a re-render can take away while this is open.
  // Every id in a drawing starts with its root's (`renameIds` at draw time).
  const t = document.createElement("template");
  t.innerHTML = fig.innerHTML;
  // The copy is for reading, and its links are not followed there, so a menu
  // open on the page is not open in it.
  t.content.querySelectorAll('g.actorPopupMenu[id$="_popup"]').forEach((m) => m.setAttribute("display", "none"));
  // Its links are not followed there, so they are not Tab stops either.
  t.content.querySelectorAll("a").forEach((a) => a.setAttribute("tabindex", "-1"));
  const svg = t.content.querySelector("svg");
  const root = original.id;
  // Under `x0zoom`, not `-zoom`: a gantt task named `zoom` is already
  // `<root>-zoom`, while mermaid never puts an `x` straight after the root
  // and `numberRepeats` counts from `x2`.
  renameIds(svg, (old) => (old.startsWith(root) ? `${root}x0zoom${old.slice(root.length)}` : null));
  els.diagramView.replaceChildren(t.content);
  // mermaid's `max-width` and `width="100%"` would hold it to the window. The
  // rest of its inline style — a background, on some diagrams — stays.
  svg.style.maxWidth = "none";
  svg.removeAttribute("width");
  svg.removeAttribute("height");
  zoomed = {
    svg,
    ratio: width && height ? width / height : 1,
    scale: 1,
    source: pre.textContent,
    look,
    path: els.content.dataset.path,
    // Opened from the keyboard: where to go back to, should a save replace it.
    back: fig.matches(":focus-visible") ? markOpenable(fig) : null,
  };
  els.diagramDialog.showModal();
  zoomDiagram(1);
  els.diagramView.scrollTo(0, 0);
  els.diagramView.focus();
}

/**
 * Close the overlay once the page no longer shows the drawing it is a copy of:
 * a save that changed the diagram, another document or panel put up behind it,
 * or the page redrawn in another look — by a redraw, or by a refresh that
 * outdated one. A save that left it alone leaves it open; another document
 * that happens to hold the same diagram does not.
 */
function closeStaleDiagram() {
  if (!zoomed) return;
  const here = !els.content.hidden && els.content.dataset.path === zoomed.path;
  const figs = here ? diagramFigures() : [];
  const same = (f) => {
    const { pre, look } = diagramBlocks.get(f);
    return look === zoomed.look && pre.textContent === zoomed.source;
  };
  if (!figs.some(same)) els.diagramDialog.close();
}

function onDiagramClose() {
  // Escape mid-drag: no pointerup is coming to let go of the pointer. Whatever
  // happens below, the drag belonged to the overlay that closed.
  diagramPan.cancel();
  // The event comes a task after `close()`: an overlay opened again in between
  // is a new one, and emptying it would leave a blank modal.
  if (els.diagramDialog.open) return;
  const back = zoomed?.back;
  const path = zoomed?.path;
  zoomed = null;
  els.diagramView.replaceChildren();
  // The dialog hands focus back to the diagram it was opened from — unless
  // a save has since replaced it, and focus has fallen out of the page.
  // Only on the same document: another one put up behind the overlay
  // closed it, and its diagrams are not this one's.
  if (
    back &&
    !els.content.contains(document.activeElement) &&
    !els.content.hidden &&
    els.content.dataset.path === path
  )
    refocusOpenable(back);
}

function onDiagramWheel(event) {
  if (!zoomed || !event.ctrlKey) return;
  event.preventDefault();
  const dy = event.deltaMode === 1 ? event.deltaY * 16 : event.deltaY;
  zoomDiagram(zoomed.scale * Math.exp(-dy * 0.002), event.clientX, event.clientY);
}

function onDiagramDblClick(event) {
  if (!zoomed) return;
  zoomDiagram(zoomed.scale === 1 ? 2.5 : 1, event.clientX, event.clientY);
}

function onDiagramToolsClick(event) {
  const what = event.target.closest("button[data-zoom]")?.dataset.zoom;
  if (!zoomed || !what) return;
  if (what === "in") zoomDiagram(zoomed.scale * ZOOM_STEP);
  else if (what === "out") zoomDiagram(zoomed.scale / ZOOM_STEP);
  else if (what === "fit") zoomDiagram(1);
  else if (what === "close") {
    // The dialog hands focus back to the diagram it was opened from, and a
    // key press puts its ring back: after a mouse close, a Space meant to
    // scroll would open it again. Let go of it, as `onLinkClick` does, and
    // of the way back, or `onDiagramClose` takes the focus that left the
    // page for a save's doing and puts it back.
    if (event.detail > 0) zoomed.back = null;
    els.diagramDialog.close();
    if (event.detail > 0 && diagramBlocks.has(document.activeElement)) document.activeElement.blur();
  }
}

/** The same Ctrl spellings the picture tab takes. Escape is the dialog's own. */
function onDiagramKeydown(event) {
  if (!zoomed || !(event.ctrlKey || event.metaKey) || event.altKey) return;
  if (event.key === "+" || event.key === "=") zoomDiagram(zoomed.scale * ZOOM_STEP);
  else if (event.key === "-") zoomDiagram(zoomed.scale / ZOOM_STEP);
  else if (event.key === "0") zoomDiagram(1);
  else return;
  event.preventDefault();
}

/**
 * Put a copy button on every heading comrak gave a line to. The line is what
 * `section_source` needs to find the section again, so a heading without one
 * gets no button.
 */
function addCopyButtons(root) {
  root.querySelectorAll("h1, h2, h3, h4, h5, h6").forEach((heading) => {
    if (!heading.dataset.sourcepos) return;
    const btn = document.createElement("button");
    btn.type = "button";
    btn.className = "copy-section";
    btn.setAttribute("aria-label", "Copy section as Markdown");
    btn.title = "Copy section as Markdown";
    btn.appendChild(els.copyIcon.content.cloneNode(true));
    heading.appendChild(btn);
  });
}

function show(which) {
  els.content.hidden = which !== "content";
  els.empty.hidden = which !== "empty";
  els.image.hidden = which !== "image";
  els.error.hidden = which !== "error";
  // Opening anything writes this window's session over the one on offer, so
  // the offer is stale from then on and must not come back with the empty
  // screen when that tab closes.
  if (which !== "empty") els.emptyReopenBtn.hidden = true;
  // The image panel brings its own scroll box. Left to itself the body would
  // scroll too, giving two scrollbars for one thing to scroll.
  document.documentElement.classList.toggle("image-mode", which === "image");
  // Anything else on screen means there is no picture to zoom, and every
  // handler that reads `picture` checks it first.
  if (which !== "image") picture = null;
  // A panel has no entry; a page's is set by `showActive` once the page is in.
  if (which !== "content" && which !== "image") shownEntry = null;
  // Called once the new page is in, so its diagrams are there to match.
  closeStaleDiagram();
}

let toastTimer;
/** Say something briefly without leaving the page: shown for a few seconds, or until clicked. */
function toast(message) {
  els.toast.textContent = String(message);
  els.toast.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (els.toast.hidden = true), 4000);
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
  // A box ticked from the keyboard comes back as a new element when the
  // watcher re-renders; put focus back on it — but only if it is still the
  // same task, or the next Space ticks a neighbour. Only a keyboard focus
  // (`:focus-visible`): a clicked box is focused too, and bringing that back
  // would let a Space meant to scroll untick it. Focus in the sidebar or a
  // dialog stays where it is. Read before `dataset.path` is overwritten below.
  const f = document.activeElement;
  const ticked =
    f?.type === "checkbox" &&
    f.matches(":focus-visible") &&
    els.content.contains(f) &&
    doc.path === els.content.dataset.path
      ? f.closest("li[data-sourcepos]")
      : null;
  const refocus = ticked && { pos: ticked.dataset.sourcepos, text: ticked.textContent };
  // A diagram or picture with focus is put back the same way, but with no
  // `:focus-visible` test: a save from another app lands while this window is
  // inactive, where it reads false. A mouse press never leaves focus on one
  // here — the mousedown listener and `onLinkClick` both let go of it.
  const openable =
    f?.matches(OPENABLE) &&
    els.content.contains(f) &&
    doc.path === els.content.dataset.path
      ? markOpenable(f)
      : null;

  clearHash();

  els.content.innerHTML = doc.html;
  els.content.dataset.path = doc.path;
  els.content.dataset.stamp = doc.stamp ?? "";
  // comrak marks every task box disabled; here they are live, because a click
  // goes back to the file — unless the file cannot be written back. A file
  // decode had to repair is not valid UTF-8, so its boxes stay as comrak left
  // them.
  if (doc.editable)
    for (const box of els.content.querySelectorAll('li > input[type="checkbox"]'))
      box.disabled = false;
  resolveMedia(els.content, doc.dir, doc.repo);
  wrapTables(els.content);
  renderDiagrams(els.content);
  highlight(els.content);
  addCopyButtons(els.content);
  show("content");
  if (refocus) {
    const again = els.content.querySelector(`li[data-sourcepos="${refocus.pos}"]`);
    if (again?.textContent === refocus.text)
      again.querySelector(':scope > input[type="checkbox"]')?.focus({ preventScroll: true });
  }
  if (openable) refocusOpenable(openable);
  // Back on the document a picture was opened from with the keyboard: onto
  // that picture again. Cleared on every render, this document's or another's,
  // so a stash whose tab went elsewhere never fires on a later, unrelated one.
  if (pictureBack?.path === doc.path) refocusOpenable(pictureBack.mark);
  pictureBack = null;

  // Restore after layout, so the offset being scrolled to actually exists yet —
  // unless by then another render has been shown, or the page has passed to
  // another entry (a picture's panel, an anchor sibling), whose scroll this is not.
  const token = renderToken;
  const entry = shownEntry;
  requestAnimationFrame(() => {
    if (token !== shownToken || shownEntry !== entry) return;
    /*
     * Arriving through a cross-file link: land on the section it named. Only
     * on arrival — `scrollY` is null exactly when nothing has been recorded
     * yet, and once this entry has a position of its own, Back and Forward
     * must return to that rather than jumping to the anchor a second time.
     */
    if (hash && scrollY == null && jumpToAnchor(hash)) {
      bankLanding(hash);
      return;
    }
    window.scrollTo(0, scrollY ?? 0);
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
const PAN_THRESHOLD = 3; // px before a click on the picture or the diagram becomes a pan

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
 * Resize `el` inside the scroll box `view` about a point, so whatever was
 * under the cursor stays under it — or the box's centre, given no point.
 * Done by measuring before and after rather than by arithmetic on offsets:
 * the content is centred while it is smaller than the box and hard against
 * the edge once it is bigger, and measuring is right either way. `resize`
 * applies the new size.
 */
function zoomAbout(view, el, resize, clientX, clientY) {
  const box = view.getBoundingClientRect();
  const before = el.getBoundingClientRect();
  const ax = clientX ?? box.left + box.width / 2;
  const ay = clientY ?? box.top + box.height / 2;
  const fx = before.width ? (ax - before.left) / before.width : 0.5;
  const fy = before.height ? (ay - before.top) / before.height : 0.5;
  resize();
  const after = el.getBoundingClientRect();
  view.scrollLeft += after.left + fx * after.width - ax;
  view.scrollTop += after.top + fy * after.height - ay;
}

function zoomTo(w, clientX, clientY) {
  if (!picture) return;
  zoomAbout(els.imageView, els.imageEl, () => applyWidth(w), clientX, clientY);
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
  // On the entry on screen, for the reason `rememberScroll` gives. `picture`
  // is null until one has decoded: before that its zoom is not this entry's.
  const entry = shownEntry;
  if (!entry || !picture) return;
  entry.scale = picture.fit ? null : picture.width / picture.base;
  entry.scrollLeft = els.imageView.scrollLeft;
  entry.scrollTop = els.imageView.scrollTop;
}

async function showImage(asset, entry, token) {
  // The strip may have just appeared or gone; the panel is sized against the
  // bar, so settle its height before anything is measured against it.
  renderTabs();

  // Until this one has decoded, `picture` would still describe the last, and
  // a zoom in that gap would be worked out from the wrong size and banked on
  // this entry.
  picture = null;
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
  if (zoomed) zoomDiagram(zoomed.scale);
  if (!picture) return;
  const scale = picture.base ? picture.width / picture.base : 1;
  zoomTo(picture.fit ? defaultWidth() : scale * baseWidth());
}

/** Render whatever the active tab points at. `scrollY` overrides the saved spot. */
async function showActive(scrollY) {
  const tab = activeTab();
  // Before the empty screen too: closing the last tab while it is still
  // loading must outdate that load, or it paints the closed document over this.
  const token = ++renderToken;
  if (!tab) {
    shownToken = token;
    show("empty");
    updateChrome();
    followTab(null);
    return;
  }

  const entry = currentEntry(tab);
  // Whether the page being left has been replaced yet.
  let swapped = false;
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
      // Always refetched: the webview may be holding bytes from before the
      // file changed, and re-reading a local picture costs next to nothing.
      bumpAsset(asset.path);
      // The page being left, as it stands at the swap: a refresh restores
      // from this, and a switch leaves it on its own entry.
      rememberScroll();
      // Its panel goes up before the picture decodes, and the page left is gone.
      shownEntry = entry;
      swapped = true;
      await showImage(asset, entry, token);
      if (token !== renderToken) return;
    } else {
      // `extent` is how far a JSON document had been loaded — undefined on
      // everything else, which Tauri drops from the call as `None`.
      const doc = await invoke("load_file", { path: entry.path, extent: entry.extent });
      if (token !== renderToken) return; // a newer switch already won
      // Drawn before the page is swapped, so it lands at its full height and
      // the scroll restore below it is exact.
      await warmDiagrams(doc.html, () => token !== renderToken);
      if (token !== renderToken) return;
      entry.path = doc.path;
      tab.path = doc.path;
      tab.dir = doc.dir;
      tab.repo = doc.repo;
      tab.label = baseName(doc.path);
      tab.heading = doc.title ?? "";
      // The page being left, as it stands at the swap — a scroll in the last
      // frame has not reached the listener yet. A refresh lands on this, and a
      // switch leaves it on its own entry.
      rememberScroll();
      swapped = true;
      shownEntry = entry; // before the render, which ties its scroll restore to it
      renderDocument(doc, scrollY ?? entry.scrollY, entry.hash);
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
    // Before the swap the page on screen is still the one being left, and a
    // scroll in its last frame has not reached the listener yet: bank it before
    // the error panel collapses it. Past the swap the offset on screen is the
    // new, half-rendered page's, and the page left was banked at the swap.
    if (!swapped) rememberScroll();
    show("error");
  }
  shownToken = token;
  syncWatch();
  updateChrome();
  // After the load, not before: `tab.dir` is canonical by now, and a render a
  // newer switch superseded has already returned without touching the sidebar.
  // Not awaited: nothing a tab change leads to should wait on a folder listing.
  followTab(tab).catch(console.error);
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
  // The funnel every tab change already reaches: opening, activating, closing
  // and navigating all end up here, so the saved session follows them without
  // each of them having to remember to say so. Without delay when it can be:
  // the reader can quit inside one.
  reportSoon();
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
    const active = t.id === activeId;
    el.className = "tab" + (active ? " active" : "");
    el.dataset.id = String(t.id);
    el.setAttribute("role", "tab");
    el.setAttribute("aria-selected", String(active));
    // Only the active tab is in the tab order, as a tablist should be.
    el.tabIndex = active ? 0 : -1;
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
/**
 * The last `watch_files` call. The command runs off the main thread, so two
 * calls could finish out of order and the older watcher would win; each waits
 * for the one before, so they cannot.
 */
let watchCall = Promise.resolve();

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
  // Skipped if a newer call is queued behind it: that one installs the set
  // that counts, and on a share that has gone away each call can take ~21 s.
  watchCall = watchCall.then(
    () => watching === key && invoke("watch_files", { paths }).catch(console.error),
  );
}

/** Whether a change to this file would be reported, or would pass unnoticed. */
function isWatched(file) {
  return watched.has(normPath(file));
}

/** How long reports are held apart, so a run of changes costs one write. */
const REPORT_DELAY = 500;
let reportTimer = null;
let lastReport = 0;

/**
 * Tell Rust what this window has open — which it writes down, so that both an
 * update restart and an ordinary launch can bring it back. The same shape a
 * tab travels in between windows.
 */
function reportSession() {
  clearTimeout(reportTimer);
  lastReport = Date.now();
  return invoke("set_session", {
    tabs: tabs.map(packTab),
    active: Math.max(0, tabs.findIndex((t) => t.id === activeId)),
    sidebar: state.folder !== null,
    // Nothing still loading. A restored window is only restored once its
    // document is on screen, and a focus or move can report before that.
    settled: shownToken === renderToken,
  }).catch(console.error);
}

/** Report a moment after things settle — for the changes that come in runs. */
function scheduleReport() {
  clearTimeout(reportTimer);
  reportTimer = setTimeout(reportSession, REPORT_DELAY);
}

/**
 * Report now if it has been quiet, and a moment after things settle if it has
 * not. Nothing fires on a Quit from outside the app (Dock, app switcher) or a
 * kill, so a tab closed just before one has to be on disk already or it comes
 * back — but a document being rewritten under the reader lands here several
 * times a second, and that must not become several writes a second.
 */
function reportSoon() {
  if (Date.now() - lastReport >= REPORT_DELAY) reportSession();
  else scheduleReport();
}

async function openTab(path) {
  const tab = makeTab(path);
  // A tab opened on a file under the folder on show keeps that root, so a link
  // or a picture clicked inside a project does not re-root its tree. One that
  // lies outside it follows its own folder instead.
  const root = state.folder ?? sidebarTab?.folder;
  if (root && isInside(path, root)) {
    tab.folder = root;
    // Picked stays picked: the reader chose this root, and a tab opened under
    // it is one more document of the same project.
    tab.picked = sidebarTab ? sidebarTab.picked && sidebarTab.folder === root : state.folderPicked;
    // Shared, not copied: an `expanded` array is only ever replaced, never
    // written into, so the two tabs on one tree cannot surprise each other.
    // Only where that list is this root's: a sidebar borrowing a folder while
    // the tab's own will not list holds another folder's.
    if (sidebarTab && sidebarTab.folder === root) tab.expanded = sidebarTab.expanded;
    // The search too: a hit opened beside the rest must not cost the others.
    if (state.folder !== null) tab.filter = els.treeFilter.value;
  }
  tabs.push(tab);
  activeId = tab.id;
  syncWatch();
  await showActive(0);
}

async function activateTab(id) {
  if (id === activeId) return;
  activeId = id;
  await showActive();
}

/** Drop a tab without recording it as closed — used when it moves elsewhere. */
async function removeTab(id) {
  const i = tabs.findIndex((t) => t.id === id);
  if (i < 0) return null;
  const wasActive = id === activeId;
  if (wasActive) rememberScroll();
  const [gone] = tabs.splice(i, 1);
  if (wasActive) activeId = (tabs[i] ?? tabs[i - 1])?.id ?? null;
  syncWatch();
  // Said now rather than left to `updateChrome`: for the active tab that waits
  // on the next document loading, and the reader can quit before it has.
  reportSoon();
  // Losing a background tab changes nothing about what is on screen, and
  // re-rendering would reload the active document and drop the reader back at
  // its last recorded position rather than where they actually are.
  if (wasActive) await showActive();
  else updateChrome();
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
  const { path, entries, index, folder, picked, sort, expanded } = tab;
  return { path, entries, index, folder, picked, sort, expanded };
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
  const tab = Object.assign(makeTab(path), { entries, index });
  // Hand-edited session files and older ones alike: anything that is not a
  // folder or one of the two orders leaves `makeTab`'s default in place.
  if (typeof data.folder === "string") tab.folder = data.folder;
  if (data.sort === "name" || data.sort === "modified") tab.sort = data.sort;
  tab.picked = data.picked === true && tab.folder !== null;
  if (Array.isArray(data.expanded)) tab.expanded = data.expanded.filter((p) => typeof p === "string");
  return tab;
}

/** Rebuild a tab handed over from another window and take it on. */
async function adoptTab(data, at) {
  const tab = rebuildTab(data);
  if (!tab) return;

  tabs.splice(Math.max(0, Math.min(tabs.length, at)), 0, tab);
  activeId = tab.id;
  syncWatch();
  await showActive();
  appWindow.setFocus().catch(() => {});
}

/**
 * Put back every tab an update restart carried over, in order, and the sidebar
 * if the window had it open.
 */
async function restoreTabs(list, active, sidebar) {
  const rebuilt = list.map(rebuildTab);
  // A tab that got here first — a file opened or a tab dropped once
  // `take_pending` had answered — stays, and stays in front: the reader has
  // just asked for it.
  const early = tabs.length > 0;
  tabs = [...rebuilt.filter(Boolean), ...tabs];
  syncWatch();
  // The early tab is already on screen: showing it again would load it twice
  // and put it back at the last scroll position banked, not where it is.
  if (early) updateChrome();
  else {
    // `active` counts the list as it was saved. A tab that names nothing any
    // more rebuilds to null and shifts everything after it, so the one to
    // activate is picked before the filter, not after.
    activeId = (rebuilt[active] ?? tabs[0])?.id ?? null;
    await showActive();
  }
  // Opened the way the sidebar button opens it, for whichever tab is in
  // front — the early one too: the sidebar belongs to the window, and a
  // file joining it adds a tab without closing anything. Only with a folder
  // to name, so a restore never puts a picker up; and not if the reader has
  // opened it already.
  if (sidebar && state.folder === null && activeDir()) await showFolder();
}

/* ---------------- folder sidebar ---------------- */

/**
 * The tab the sidebar is showing for. The object, not its id: writing onto a
 * closed tab is harmless, and means Ctrl+Shift+T brings its folder back with it.
 */
let sidebarTab = null;

/**
 * Renders of the root still out, whoever asked: a folder opening, a new order,
 * the watcher. While any is, the tree is half-built — one that a later render
 * outdated may still be reopening the folders inside it — so `syncFolderWatch`
 * waits for the last. Only this folder's count: `openFolder` starts a new epoch
 * for a different one, or a share that never answers would hold up the rest.
 */
let folderLoads = 0;
let rootEpoch = 0;

/**
 * What the root renders still out were asked to reopen. One that outdates
 * another takes this on, or a save landing mid-switch would bring the tree back
 * shut and have that written down as what the tab remembers.
 */
let rootKeep = new Set();

/**
 * The folder the sidebar owes `tab`, for a tab switch and for Ctrl+Shift+O
 * alike. One the reader picked always comes back, wherever the tab has been
 * since. One the sidebar only followed to comes back while the document still
 * lies inside it; after that the tab stays on the folder on show if the
 * document is in there — the rule `openTab` gives a new tab — and otherwise
 * goes to the document's own.
 */
function folderFor(tab) {
  const doc = currentEntry(tab)?.path;
  const holds = (dir) => !!dir && !!doc && isInside(doc, dir);
  if (tab.folder && (tab.picked || !doc || holds(tab.folder))) return tab.folder;
  if (holds(state.folder)) return state.folder;
  return tab.dir || dirName(tab.path);
}

/**
 * Move the sidebar to the tab now on screen: the folder `folderFor` names. The
 * only place `sidebarTab` changes.
 */
async function followTab(tab) {
  if (tab === sidebarTab) return; // in-tab navigation leaves the sidebar alone
  sidebarTab = tab;
  if (!tab) return; // empty window: the sidebar stays as it is
  showSortMenu(false); // left open, it would go on ticking the last tab's order
  const folder = folderFor(tab);
  // A load that failed on a bare name: nowhere to go. Before the order is
  // taken, or the tree would stay in the old one with the menu ticking the new.
  if (!folder) return;
  const resort = tab.sort !== state.folderSort;
  state.folderSort = tab.sort;
  if (state.folder === null) return; // closed: showFolder picks it up later
  // What the tab remembers is about its own folder, not one standing in for it.
  const own = folder === tab.folder;
  if (!sameDir(folder, state.folder)) {
    return openFolder(
      folder,
      own
        ? { keep: tab.expanded, filter: tab.filter, picked: tab.picked, record: false }
        : { record: false },
    );
  }
  if (tab.folder !== state.folder) {
    tab.folder = state.folder;
    reportSoon(); // `updateChrome` has already said its piece for this switch
  }
  // `openFolder` is what usually says this, and it has not run: a tab opened
  // into an empty window reads it to learn whether this folder was chosen.
  state.folderPicked = own && tab.picked;
  els.treeFilter.value = tab.filter; // the sync below is what applies it
  if (resort) await resortTree();
  else syncFolderWatch(); // also records what is expanded onto the new owner
}

/** Per-list render tokens: a watcher burst and a click can re-list the same folder. */
const treeTokens = new WeakMap();
/** What each list last showed, so a save that changes nothing does not rebuild it. */
const treeListings = new WeakMap();

/**
 * One level at a time. A list is rebuilt whenever its folder is expanded or
 * the watcher reports a change in it, and whatever was expanded inside it is
 * expanded again afterwards, so a re-list never costs the user their place.
 *
 * `keep` is what a rebuilt ancestor saw open below here. This list is new and
 * empty by the time it is asked, so it cannot see that for itself, and without
 * it a rebuild would reopen one level and leave everything deeper shut.
 */
async function renderTree(ul, dir, keep = new Set()) {
  if (ul !== els.tree) return listTree(ul, dir, keep);
  const epoch = rootEpoch;
  folderLoads++;
  for (const path of keep) rootKeep.add(path);
  // After a listing that failed there are no rows left to read what was open
  // off, and a tree that came back shut would be written down as what the tab
  // remembers. The tab still knows: `syncFolderWatch` left it alone.
  if (
    els.sidebar.dataset.tree === "error" &&
    sidebarTab?.folder &&
    sameDir(sidebarTab.folder, dir)
  ) {
    for (const path of sidebarTab.expanded) rootKeep.add(path);
  }
  try {
    return await listTree(ul, dir, rootKeep);
  } finally {
    // One from before the folder changed was written off when it did.
    if (epoch === rootEpoch && !--folderLoads) rootKeep = new Set();
  }
}

/**
 * Re-list the root in the order now in force. It is all that needs asking: the
 * sort is part of every list's signature, so the root rebuilds, and rebuilding
 * is what re-lists each folder open inside it, all the way down. Rebuilt rows
 * come back unhidden, and the sync is what puts the filter back on.
 */
async function resortTree() {
  await renderTree(els.tree, els.tree.dataset.dir);
  syncFolderWatch();
}

/**
 * The listing itself. `renderTree` is the way in: it keeps count of the root's.
 * Resolves `false` when this listing failed, even if a newer one has the tree:
 * a caller overtaken meanwhile still has to know how its own pick went. And
 * `"same"` when the listing had not changed, so nothing was rebuilt.
 */
async function listTree(ul, dir, keep) {
  const token = (treeTokens.get(ul) ?? 0) + 1;
  treeTokens.set(ul, token);
  ul.dataset.dir = dir;

  const sort = state.folderSort; // read once: it can change while the listing is out
  let listing;
  try {
    listing = await invoke("list_dir", { path: dir, sort });
  } catch (err) {
    if (token !== treeTokens.get(ul)) return false;
    treeListings.delete(ul);
    const li = document.createElement("li");
    li.className = "tree-row error";
    li.textContent = String(err);
    ul.replaceChildren(li);
    if (ul === els.tree) els.sidebar.dataset.tree = "error";
    return false;
  }
  if (token !== treeTokens.get(ul)) return; // a newer listing already won
  ul.dataset.dir = listing.dir;
  const { entries } = listing;

  // The watcher reports every save in the folder, and a save changes nothing
  // the tree shows. Rebuilding anyway would collapse-and-reopen the subtree.
  // The sort is part of it: a list whose own order comes out the same under a
  // new sort still has to rebuild, because that is what re-lists the folders
  // open inside it.
  const signature = sort + JSON.stringify(entries);
  if (signature === treeListings.get(ul)) return "same";
  treeListings.set(ul, signature);

  // Every level, not just this one: the lists below are about to be replaced.
  const expanded = new Set([
    ...keep,
    ...[...ul.querySelectorAll('li[aria-expanded="true"] > .tree-row')].map((r) => r.dataset.path),
  ]);

  const nodes = entries.map((e) => {
    const li = document.createElement("li");
    li.setAttribute("role", "treeitem");
    li.tabIndex = -1; // markTreeSelection puts the selected row in the tab order

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
      li.setAttribute("aria-expanded", "false"); // belongs on the treeitem, not the row inside it
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
  if (ul === els.tree) els.sidebar.dataset.tree = entries.length ? "" : "empty";
  markTreeSelection();

  const reopen = [...ul.querySelectorAll(":scope > li > .tree-row[data-dir]")].filter((r) =>
    expanded.has(r.dataset.path),
  );
  await Promise.all(reopen.map((r) => expandRow(r, true, expanded)));
}

/**
 * Bank the search on the tab on screen — while the sidebar shows that tab's
 * own folder, or the tab has none yet. Not while it is borrowing the
 * document's folder because its own will not list: nothing about a stand-in
 * is written down, the rule `syncFolderWatch` keeps for `expanded`.
 */
function rememberFilter(value) {
  const tab = activeTab();
  if (!tab || state.folder === null) return;
  if (!tab.folder || sameDir(tab.folder, state.folder)) tab.filter = value;
}

/**
 * Narrow the tree to rows whose name contains what is in the filter box. A
 * folder that matches shows everything under it; one that does not stays for
 * any loaded descendant that does, collapsed or not, so the hit is a click away.
 */
function applyTreeFilter() {
  const q = els.treeFilter.value.trim().toLowerCase();
  const pass = (ul, inherited) => {
    let any = false;
    for (const li of ul.children) {
      const name = li.querySelector(":scope > .tree-row > .tree-name");
      // An error row has no name and always stays.
      const hit = inherited || !name || name.textContent.toLowerCase().includes(q);
      const sub = li.querySelector(":scope > ul");
      const kid = sub ? pass(sub, hit) : false;
      li.hidden = !hit && !kid;
      any ||= !li.hidden;
    }
    return any;
  };
  pass(els.tree, !q);
  setTreeStop(); // the filter may just have hidden the row that was it
}

/** Open or close a folder row. Leaves the watcher alone — see syncFolderWatch. */
async function expandRow(row, open, keep) {
  row.parentElement.setAttribute("aria-expanded", String(open));
  const children = row.nextElementSibling;
  children.hidden = !open;
  if (!open) return setTreeStop(); // the tab stop may have been inside
  // Unchanged here, but nothing watched the folders left open inside while
  // this one was shut: ask each again. A rebuild re-lists them itself.
  if ((await renderTree(children, row.dataset.path, keep)) !== "same") return;
  const inner = children.querySelectorAll(':scope > li[aria-expanded="true"] > .tree-row');
  await Promise.all([...inner].map((r) => expandRow(r, true)));
}

let watchingFolders = null;
/** The last `watch_folders` call, chained for the reason `watchCall` is. */
let folderWatchCall = Promise.resolve();

/**
 * Keep the Rust watcher on exactly the folders on show: the root and every
 * expanded row that is not itself inside a collapsed one. Called once a change
 * to the tree has settled, never from the middle of a rebuild — a half-built
 * list would look emptier than it is, and dropping a watch to re-add it a
 * moment later loses whatever happened in between.
 */
function syncFolderWatch() {
  // Every root render ends in a call here, so the last one out does the work.
  // A closed sidebar has nothing half-built about it.
  if (folderLoads && state.folder !== null) return;
  applyTreeFilter(); // the same settled moment is when the filter wants the finished tree
  const dirs = [];
  if (state.folder !== null) {
    dirs.push(state.folder);
    for (const row of els.tree.querySelectorAll('li[aria-expanded="true"] > .tree-row')) {
      if (!row.closest("ul[hidden]")) dirs.push(row.dataset.path);
    }
    // The settled tree, minus its root: exactly what the owning tab has to
    // reopen with. Not off a listing that failed: an unplugged drive shows
    // nothing open, and that is no reason to forget what was. Nor off a folder
    // the tab is only borrowing while its own will not list — nothing of that
    // one is written down.
    if (
      sidebarTab &&
      els.sidebar.dataset.tree !== "error" &&
      sidebarTab.folder &&
      sameDir(sidebarTab.folder, state.folder)
    ) {
      const open = dirs.slice(1);
      // Part of the session too, but only said when it changed: the watcher
      // lands here on every save in the folder.
      const changed = open.join("\0") !== sidebarTab.expanded.join("\0");
      sidebarTab.expanded = open;
      if (changed) reportSoon();
    }
  }
  const key = dirs.join("\0");
  if (key === watchingFolders) return;
  watchingFolders = key;
  folderWatchCall = folderWatchCall.then(
    () => watchingFolders === key && invoke("watch_folders", { dirs }).catch(console.error),
  );
}

/**
 * Each tab's folder as it stood before the `openFolder` calls still out for it,
 * and how many there are. A pick that fails goes back to this, not to what an
 * overtaken call wrote onto the tab before its own listing had answered.
 */
const folderBefore = new Map();

/**
 * Show `path` in the sidebar. `keep` is what was expanded last time this folder
 * was on show and `filter` what the filter box held, if anything. `record` says
 * the reader asked for this folder, so it is worth keeping as the one a picker
 * starts in; `remember` whether the tab takes it on as its own, and `picked`
 * whether as one the reader chose rather than one the sidebar followed to.
 */
async function openFolder(
  path,
  { keep, filter = "", record = true, remember = true, picked = false } = {},
) {
  const last = state.folder;
  state.folder = path;
  state.folderPicked = picked;
  // Whether the sidebar is open is part of the session. Opening it on the
  // folder the tab already had is news the report further down never sees.
  if (last === null) reportSoon();
  const owner = activeTab(); // not `sidebarTab`, which lags a loading switch
  const writes = !!owner && remember;
  const before = (writes && folderBefore.get(owner)) || {
    folder: owner?.folder,
    picked: owner?.picked,
    filter: owner?.filter,
    out: 0,
    n: 0, // calls made for this tab while any is out; the last one answers for it
  };
  const n = writes ? ++before.n : 0;
  if (writes) {
    before.out++;
    folderBefore.set(owner, before);
  }
  const { folder: was, picked: wasPicked, filter: wasFilter } = before;
  els.sidebar.hidden = false;
  els.treeFilter.value = filter;
  // Onto the tab before the listing, not after it: a switch away while it is
  // out would otherwise lose what the reader just picked.
  if (owner && remember) Object.assign(owner, { folder: path, picked, filter });
  treeListings.delete(els.tree); // a different folder must rebuild even if it lists the same
  els.sidebar.dataset.tree = ""; // the last folder's error or emptiness is not this one's
  // Emptied first: a rebuild reopens whatever the old rows had open, and when
  // this folder sits inside the last one, that is another tab's memory. Not
  // for the folder already on show — picked again, it keeps what it has open.
  if (last === null || !sameDir(path, last)) {
    els.tree.replaceChildren();
    // And so is whatever a render of the last folder was carrying, and the
    // render itself: it no longer counts towards this tree being settled.
    rootKeep = new Set();
    rootEpoch++;
    folderLoads = 0;
  }
  let listed;
  try {
    listed = await renderTree(els.tree, path, new Set(keep));
  } finally {
    if (writes && !--before.out) folderBefore.delete(owner);
  }
  // Overtaken by a newer call, which has the sidebar now but not necessarily
  // this tab: a switch away takes the sidebar to another. A pick whose own
  // listing failed was never really picked, so the tab goes back — unless
  // something newer has written it since. Closed is handled below.
  const overtaken = state.folder !== null && state.folder !== path;
  const latest = writes && before.n === n;
  if (latest && overtaken && listed === false && owner.folder === path) {
    Object.assign(owner, { folder: was, picked: wasPicked, filter: wasFilter });
    reportSoon();
  }
  if (state.folder === null) {
    // Closed while the listing was out. A folder whose own listing failed was
    // never really picked, so the tab goes back to the one it had — unless a
    // newer call has answered for it since. Not the filter: closing blanked it.
    if (latest && listed === false && owner.folder === path) {
      Object.assign(owner, { folder: was, picked: wasPicked });
      reportSoon();
    }
    return;
  }
  // Canonical from here on, so it compares with what the watcher reports.
  if (state.folder === path) {
    state.folder = els.tree.dataset.dir;
    const failed = els.sidebar.dataset.tree === "error";
    // A folder that will not list leaves the tab exactly as it was: one the
    // reader had is kept, and a fresh pick that failed was never really picked.
    if (owner && remember) {
      owner.folder = failed ? was : state.folder;
      if (failed) Object.assign(owner, { picked: wasPicked, filter: wasFilter });
    }
    // A folder the reader picked is part of what the session holds. A tab
    // switch coming back to the one it left has nothing new to say.
    if (owner && (owner.folder !== was || owner.picked !== wasPicked)) reportSoon();
    // Fire-and-forget, like `reportSession`: nothing on screen waits for it.
    // Not when the listing failed, because `dataset.dir` is then still the path
    // that failed — recording an unplugged drive would lose the folder that
    // works, and leave the next picker opening in home instead.
    if (record && state.folder && !failed && listed !== false) {
      invoke("set_last_folder", { path: state.folder }).catch(console.error);
    }
    // The tab's own folder, unplugged: show the document's folder this time
    // rather than an error row, without writing any of it down. The tab still
    // remembers the drive, and gets it back when the drive comes back. With
    // nowhere else to go the error stays on show, folder still remembered.
    if (failed && owner && was && sameDir(was, path)) {
      const dir = activeDir();
      if (dir && !sameDir(dir, path)) return openFolder(dir, { record: false, remember: false });
    }
  }
  els.sidebarNameText.textContent = baseName(state.folder);
  els.sidebarName.title = `${state.folder}\nClick to show a different folder`;
  syncFolderWatch();
}

function closeFolder() {
  state.folder = null;
  els.sidebar.hidden = true;
  // The menu floats over the document rather than inside the sidebar, so it
  // would otherwise be left pointing at a button that is no longer there.
  showSortMenu(false);
  els.treeFilter.value = "";
  // Every tab's, closed ones too: shutting the sidebar dismisses the search,
  // and one coming back with the next tab would narrow the tree unasked.
  for (const tab of [...tabs, ...closedTabs]) tab.filter = "";
  els.tree.replaceChildren();
  syncFolderWatch();
  reportSoon(); // whether the sidebar is open is part of the session
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
    const on = !!path && !row.dataset.dir && samePath(row.dataset.path, path);
    row.classList.toggle("active", on);
    // The state is the treeitem's, not the row's.
    const li = row.parentElement;
    if (on) li.setAttribute("aria-selected", "true");
    else li.removeAttribute("aria-selected");
  }
  setTreeStop();
}

/** The treeitems on show, in order: what the arrow keys walk. */
function visibleTreeItems() {
  return [...els.tree.querySelectorAll('li[role="treeitem"]')].filter((li) => !li.closest("[hidden]"));
}

/**
 * Keep exactly one treeitem in the tab order, as a tree should have: the one
 * with focus, else the selected one, else the first on show. Only one on show
 * will do — Tab skips a row the filter or a collapsed folder hides, and the
 * tree would drop out of the tab order altogether.
 */
function setTreeStop() {
  const items = visibleTreeItems();
  const stop =
    items.find((li) => li === document.activeElement) ??
    items.find((li) => li.getAttribute("aria-selected") === "true") ??
    items[0];
  for (const li of els.tree.querySelectorAll('li[role="treeitem"]')) li.tabIndex = li === stop ? 0 : -1;
}

async function onTreeClick(event) {
  const row = event.target.closest(".tree-row[data-path]");
  if (!row) return;
  const path = row.dataset.path;

  if (row.dataset.dir) {
    await expandRow(row, row.parentElement.getAttribute("aria-expanded") !== "true");
    syncFolderWatch();
    return;
  }

  // Ctrl+click opens beside the current document rather than in its place, and
  // Shift+click gives it a window of its own, as in a browser. Plain click
  // walks the active tab's history like a link. On the Mac the tab modifier is
  // Cmd alone: Ctrl+click there is the right-click that raises the menu, and a
  // webview that sends the click as well must not also open the file under it.
  if (isMac && event.ctrlKey) return;
  if (event.shiftKey) await invoke("open_window", { path });
  else if (event.metaKey || event.ctrlKey) await openTab(path);
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

/**
 * The tree's keys, as the WAI-ARIA tree pattern has them. Enter, Space and the
 * arrows that open or shut a folder click the row, so they do exactly what the
 * mouse does. Unmodified keys only: Alt+←/→ is Back and Forward, and the rest
 * belong to `onKeydown` too. Stopped here once handled, or an arrow would also
 * dismiss a menu there.
 */
function onTreeKeydown(event) {
  const li = event.target;
  if (li.getAttribute("role") !== "treeitem") return;
  if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
  const row = li.querySelector(":scope > .tree-row");
  const expanded = li.getAttribute("aria-expanded"); // null on a file
  const items = visibleTreeItems();
  let to = null;
  if (event.key === "ArrowDown") to = items[items.indexOf(li) + 1];
  else if (event.key === "ArrowUp") to = items[items.indexOf(li) - 1];
  else if (event.key === "ArrowRight") {
    if (expanded === "false") row.click();
    else if (expanded === "true") to = li.querySelector(':scope > ul > li[role="treeitem"]:not([hidden])');
  } else if (event.key === "ArrowLeft") {
    if (expanded === "true") row.click();
    else to = li.parentElement.closest('li[role="treeitem"]');
  } else if (event.key === "Enter" || event.key === " ") row.click();
  else return;
  event.preventDefault(); // Space and the arrows would scroll the sidebar too
  event.stopPropagation();
  to?.focus(); // the tab stop follows, from the tree's `focusin`
}

/* ---------------- the tree's right-click menu ---------------- */

/** The row the open menu belongs to, so its items know what to act on. */
let menuRow = null;

/**
 * Right-click a file for the three ways it can open — the same three the
 * modifiers give, said out loud. Folders get nothing: the row itself is the
 * only thing you can do to one.
 */
function onTreeContextMenu(event) {
  // Suppressed for the whole tree, not just the rows we answer: a debug build
  // would otherwise pop the webview's own menu on the ones we skip.
  event.preventDefault();

  // The keyboard (Shift+F10, Menu) aims at the focused treeitem rather than at
  // the row inside it, so start from the item either way.
  const row = event.target.closest('li[role="treeitem"]')?.querySelector(":scope > .tree-row");
  if (!row || row.dataset.dir) return showTreeMenu(null);

  showTreeMenu(row);

  // Only measurable once it is on: a hidden menu has no width to keep inside
  // the window. Shift+F10 and the Menu key carry no point to put it at — and
  // report no button either, where a real right-click reports 2 — so those fall
  // back to the row. Testing the coordinates instead would mistake a click on
  // the window's first pixel column for one of them. A Mac Ctrl+click reports
  // no button either, but it does have a point.
  const menu = els.treeMenu;
  const box = row.getBoundingClientRect();
  const fromKeyboard = event.button !== 2 && !(isMac && event.ctrlKey);
  const x = fromKeyboard ? box.left + 8 : event.clientX;
  const y = fromKeyboard ? box.bottom : event.clientY;
  menu.style.left = `${Math.max(4, Math.min(x, window.innerWidth - menu.offsetWidth - 4))}px`;
  menu.style.top = `${Math.max(4, Math.min(y, window.innerHeight - menu.offsetHeight - 4))}px`;

  menu.querySelector("button").focus();
}

/** `null` closes it. Hiding the focused item drops focus to the body, so the
 *  callers that came from the keyboard put it back on the row themselves. */
function showTreeMenu(row) {
  menuRow = row;
  els.treeMenu.hidden = !row;
}

async function onTreeMenuClick(event) {
  const item = event.target.closest("button[data-open]");
  if (!item || !menuRow) return;
  const path = menuRow.dataset.path;
  const li = menuRow.parentElement;
  showTreeMenu(null);
  li.focus();

  if (item.dataset.open === "window") await invoke("open_window", { path });
  else if (item.dataset.open === "tab") await openTab(path);
  else await loadPath(path);
}

/**
 * `undefined` toggles. Opening ticks whichever order is in force, puts the
 * menu under its button, and focuses that row — the keyboard should arrive on
 * the current answer, not above it.
 */
function showSortMenu(open) {
  const next = open ?? els.sortMenu.hidden;
  els.sortMenu.hidden = !next;
  els.sidebarSort.setAttribute("aria-expanded", String(next));
  if (!next) return;

  const menu = els.sortMenu;
  for (const item of menu.querySelectorAll("button[data-sort]")) {
    item.setAttribute("aria-checked", String(item.dataset.sort === state.folderSort));
  }

  // Only measurable once it is on: a hidden menu has no width to keep inside
  // the window. Clamped like the tree's menu, for a sidebar pushed up against
  // the bottom of a short window.
  const box = els.sidebarSort.getBoundingClientRect();
  menu.style.left = `${Math.max(4, Math.min(box.left, window.innerWidth - menu.offsetWidth - 4))}px`;
  menu.style.top = `${Math.max(4, Math.min(box.bottom + 4, window.innerHeight - menu.offsetHeight - 4))}px`;

  menu.querySelector('[aria-checked="true"]')?.focus();
}

/** Take the new order and show it. */
async function setFolderSort(sort) {
  if (sort === state.folderSort) return;
  state.folderSort = sort;
  // Onto the tab as well as the config: the file is what a new window starts
  // from, the tab keeps the order it was last read in, and a new tab takes
  // whichever is in force where it opens.
  // The tab on screen, not `sidebarTab`: while a switch is loading that is
  // still the tab being left, and `followTab` would take the order back.
  const tab = activeTab();
  if (tab) {
    tab.sort = sort;
    reportSoon();
  }
  invoke("set_folder_sort", { sort }).catch(console.error);
  if (state.folder === null) return;

  await resortTree();
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
    tab.entries.length = tab.index + 1; // drop the forward branch
    tab.entries.push({ path, scrollY: null, hash: id });
    tab.index = tab.entries.length - 1;
    syncWatch();
    await showActive();
    return;
  }
  // The document already on screen, named without a fragment. Read again where
  // it stands rather than from the top: no entry was pushed, so a jump would
  // leave no way back. Still a re-read, so clicking the open file in the tree
  // goes on retrying a load that failed.
  await refresh();
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
   * Only with that document settled on screen: not over the error panel, which
   * has no page to scroll, nor while a load is on its way to replace it.
   */
  if (from && to && samePath(from.path, to.path) && shownToken === renderToken && shownEntry === from) {
    shownEntry = to;
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
 * What Rust renders a document's own ids behind — its headings' and its
 * `<a id>` targets' — so none can take on the app's (`## Image` would wear the
 * image panel's `#image` rules). Links name them as written; the prefix goes on
 * here, at the jump.
 */
const DOC_ID_PREFIX = "user-content-";

/**
 * The element in the document that `id` names, or null. Prefixed first — no
 * app id can carry the prefix, which a test pins — then as written, then a
 * GitHub link without its prefix, since GitHub prefixes footnotes too
 * (`#user-content-fn-1`) and ours are bare. Two kinds of name go as written
 * first: one that already carries the prefix, which is GitHub's order, so
 * `#user-content-x` finds `## X` rather than `## User content X`; and a
 * footnote's, since comrak writes its own footnote links bare (`#fn-1`,
 * `#fnref-1`) and they must find the footnote rather than a heading named
 * `fn 1`. The bare names are looked for inside the document only: the app's
 * own elements come first in the page, and a link must never land on one.
 */
function docTarget(id) {
  if (!id) return null;
  const inDoc = (name) => els.content.querySelector(`#${CSS.escape(name)}`);
  const prefixed = () => document.getElementById(DOC_ID_PREFIX + id);
  const stripped = () => (id.startsWith(DOC_ID_PREFIX) ? inDoc(id.slice(DOC_ID_PREFIX.length)) : null);
  if (id.startsWith(DOC_ID_PREFIX) || /^fn(ref)?-/.test(id)) return inDoc(id) ?? prefixed() ?? stripped();
  return prefixed() ?? inDoc(id) ?? stripped();
}

/**
 * Jump the way a click on a same-page link would, rather than scrolling by
 * hand: that is what makes `:target` match, and `:target` is what keeps the
 * heading clear of the sticky bar. False when this document has no such id,
 * which is all a link into a section that has since been renamed deserves.
 */
function jumpToAnchor(id) {
  const target = docTarget(id);
  if (!target) return false;
  // Assigning the fragment the URL already carries is not a change, so the
  // browser does nothing and a second click on the same link goes nowhere.
  // Dropping it first makes every jump a jump, at no cost in history.
  clearHash();
  location.hash = target.id;
  return true;
}

/**
 * Record an in-page jump as a history entry, so Back returns to where the link
 * was clicked from rather than skipping the whole document.
 *
 * The jump itself is `jumpToAnchor`'s, run right after. It sets `location.hash`
 * rather than scrolling, which is what makes `:target` match and so keeps the
 * heading clear of the sticky bar — doing the scroll by hand would mean
 * reimplementing that offset.
 *
 * `id` is already decoded, as both callers have it: decoding again would look
 * `#a%2541` up as `aA`, find nothing and record no entry, while the jump still
 * goes to `a%41` — and Back would skip it.
 */
function pushAnchorEntry(id) {
  // A link to nothing scrolls nowhere, so it should not cost a Back press.
  if (!docTarget(id)) return;

  const tab = activeTab();
  const entry = currentEntry(tab);
  if (!entry) return;

  rememberScroll(); // the spot being left, captured before the browser moves
  tab.entries.length = tab.index + 1; // drop the forward branch
  tab.entries.push({ path: entry.path, scrollY: null, hash: id });
  tab.index = tab.entries.length - 1;
  // Still the same page, now under the new entry.
  if (shownEntry === entry) shownEntry = currentEntry(tab);
  updateChrome();

  // The jump happens after this handler returns; record where it landed so
  // Forward comes back to exactly the same place.
  bankLanding(id);
  // A refresh of this page still drawing would land on the entry it started
  // from, at that entry's spot. Draw it again for this one instead.
  if (shownToken !== renderToken) refresh().catch(console.error);
}

/**
 * Bank where a jump to `hash` lands, a frame from now, on the entry whose page
 * jumped — taken now: by the landing a tab switch may have moved the active
 * entry on, or swapped the page.
 */
function bankLanding(hash) {
  const landed = shownEntry;
  requestAnimationFrame(() => {
    if (landed?.hash === hash && shownEntry === landed) landed.scrollY = window.scrollY;
  });
}

/** Re-render the open document in place: no history entry, no scroll jump. */
async function refresh() {
  const entry = currentEntry(activeTab());
  if (!entry) return;
  // The paint lands where this entry's page stands at the swap: the scroll
  // listener banks while this loads and `showActive` banks at the swap, so the
  // reader can scroll on as its diagrams draw. Mid-switch, or over the error
  // panel, the page on screen is not this entry's, and its saved spot stands.
  // A re-saved picture keeps its path, and the webview would serve the copy it
  // already has. Documents are re-read by Rust, and their pictures are watched
  // in their own right, so nothing else needs invalidating here — refetching
  // them would leave the page short of their height when the scroll is restored.
  if (isImage(entry.path)) bumpAsset(entry.path);
  await showActive();
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

/* ---------------- diagram colours ---------------- */

/**
 * Reflect the choice and redraw what it changes. Also called when another
 * window changes it, so it must not re-broadcast.
 */
function showDiagramColours(colours) {
  for (const radio of els.diagramColourRadios) radio.checked = radio.value === colours;
  if (colours === state.diagramColours) return;
  // Blocks left as code count too: a diagram refused in one mode can draw in the other.
  const drawn = !els.content.hidden && (diagramFigures().length > 0 || els.content.querySelector(MERMAID_BLOCK));
  state.diagramColours = colours;
  if (!drawn) return;
  // The overlay's copy is of the old drawing.
  els.diagramDialog.close();
  redrawDiagrams().catch(console.error);
}

/* ---------------- reopening ---------------- */

/**
 * Reflect the setting in the dialog. There is no broadcast to answer, unlike
 * the open mode: nothing but the next launch reads this one.
 */
function showReopen(mode) {
  for (const radio of els.reopenRadios) radio.checked = radio.value === mode;
}

/** "3 tabs", "1 tab" — the offer counts two things and either can be one. */
function plural(n, thing) {
  return `${n} ${thing}${n === 1 ? "" : "s"}`;
}

/** What the last run left behind, offered rather than simply brought back. */
function showOffer(windows, tabs) {
  els.emptyReopenLabel.textContent =
    `Reopen ${plural(tabs, "tab")} in ${plural(windows, "window")}`;
  els.emptyReopenBtn.hidden = false;
}

async function reopenSession() {
  // The offer goes whatever happens next: Rust hands the session over once,
  // and pressing twice would ask for a session that is no longer there.
  els.emptyReopenBtn.hidden = true;
  const session = await invoke("restore_offered_session");
  if (!session) return;
  // The other windows are already being built, each with its own tabs; this
  // one takes the first, the same three steps the boot arm for a restart runs.
  await restoreTabs(session.tabs, session.active, session.sidebar);
  if (session.maximized) await appWindow.maximize().catch(() => {});
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
  els.settingsUpdate.hidden = !info;
  if (info) {
    els.updateBtn.title = `Version ${info.version} is available`;
    els.settingsUpdate.textContent = `Update to ${info.version}…`;
  }
  return info;
}

/**
 * Whether an update is being downloaded or installed — by any window, since
 * the progress is broadcast. The dialog's own state is not enough: it is reset
 * every time the dialog opens, and another window's never knew.
 */
let installing = false;

/**
 * Set whether an install is under way, with the two buttons that follow it.
 * Every window calls this as the broadcast reaches it, so a dialog already
 * open elsewhere stops offering a second install the moment one begins.
 */
function setInstalling(value) {
  installing = value;
  els.updateNow.disabled = value;
  // Closing the dialog does not stop a download, and the restart still comes
  // when it lands, so once one has begun the button only hides the dialog.
  els.updateLater.textContent = value ? "Hide" : "Later";
}

function showUpdateDialog() {
  const info = state.update;
  if (!info) return;

  els.updateSummary.textContent = `Version ${info.version} is available.`;
  els.updateNotes.textContent = info.notes;
  els.updateNotes.hidden = !info.notes;
  // An install already under way — begun here before the dialog was closed,
  // or in another window — is still under way: show it, and offer no second.
  els.updateProgress.hidden = !installing;
  if (installing && !els.updateProgress.textContent) els.updateProgress.textContent = "Downloading…";
  els.updateError.hidden = true;
  setInstalling(installing);

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
    openUrl(info.release_url).catch(toast);
    return;
  }

  setInstalling(true);
  els.updateError.hidden = true;
  els.updateProgress.hidden = false;
  els.updateProgress.textContent = "Downloading…";

  try {
    // Does not return when it succeeds: the app is restarted into the new
    // version, or on Windows killed outright by the installer.
    await invoke("install_update");
  } catch (err) {
    console.error(err);
    setInstalling(false);
    els.updateProgress.hidden = true;
    els.updateError.textContent = `Update failed: ${err}`;
    els.updateError.hidden = false;
  }
}

/** `null` percent means the manifest gave no size to measure against. */
function showUpdateProgress(percent) {
  if (els.updateProgress.hidden) return;
  els.updateProgress.textContent =
    percent == null ? "Downloading…" : `Downloading… ${percent}%`;
}

/** `undefined` toggles. Opening focuses the first item, as the other menus do. */
function showOpenMenu(open) {
  const next = open ?? els.openMenu.hidden;
  els.openMenu.hidden = !next;
  els.openMore.setAttribute("aria-expanded", String(next));
  if (next) els.openMenu.querySelector("button").focus();
}

/**
 * ↑ and ↓ walk a menu's items, round the ends, for all three menus. Enter and
 * Space are the items' own, being buttons; Escape and Tab are `onKeydown`'s.
 * Stopped here, or `onKeydown` would take the arrow for a key that dismisses.
 */
function onMenuKeydown(event) {
  if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
  event.preventDefault();
  event.stopPropagation();
  const items = [...event.currentTarget.querySelectorAll("button")];
  const step = event.key === "ArrowDown" ? 1 : -1;
  items[(items.indexOf(document.activeElement) + step + items.length) % items.length].focus();
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
/** Where the last torn-off tab sat, so a window that fails to build can give it back. */
let tornOffIndex = 0;

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
    // Where the tab sat before the drag, so a cancel can put it back.
    startIndex: tabs.findIndex((t) => t.id === id),
    moved: false,
    detached: false,
    lastProbe: 0,
    origin: null,
    // Every drag message to Rust is chained onto this, so they arrive in the
    // order they were issued. A fire-and-forget `drag_over` landing after the
    // `drag_cancel` that should have ended it leaves the other window with a
    // caret nothing will clear.
    ipc: Promise.resolve(),
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
    closeTab(id).catch(toast);
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
  if (event.buttons === 0) return onDragCancel(); // a release the page never heard (Alt+Tab, a UAC prompt)

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
      drag.ipc = drag.ipc.then(() => invoke("drag_cancel")).catch(() => {});
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
  drag.ipc = drag.ipc.then(() => invoke("drag_over", { x, y })).catch(console.error);
}

async function onDragEnd(event) {
  if (!drag || event.pointerId !== drag.pointerId) return;
  // `packTab` below copies the entries as they stand, and a tab leaving for
  // another window is serialised before `removeTab` would record the spot;
  // without this it arrives there at the top of the document.
  rememberScroll();
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
    reportSoon(); // the order is part of the session, and nothing else says it moved
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
    d.ipc = d.ipc.then(() =>
      invoke("drop_tab", {
        x,
        y,
        tab: packTab(tab),
        // The last tab already has a window to itself: tearing it off would only
        // swap this window for a new one and leave an empty shell behind.
        tearOff: tabs.length > 1,
      }),
    );
    const outcome = await d.ipc;
    // "adopted" — another window took it. "detached" — it became a new window.
    // "cancelled" — nowhere to go; the tab stays put.
    if (outcome === "cancelled") {
      renderTabs();
      return;
    }
    if (outcome === "adopted" || outcome === "detached") {
      // The new window is still being built and can yet fail; remember the slot
      // in case Rust hands the tab back.
      tornOffIndex = tabs.findIndex((t) => t.id === d.id);
      await removeTab(d.id);
    }
    // Handing the last tab to another window leaves nothing here worth keeping;
    // the user is already looking at the target, so close the empty shell —
    // once Rust has heard it is empty, or it would be written down as a closed
    // window still holding the tab that has just moved.
    if (outcome === "adopted" && tabs.length === 0) {
      await reportSession();
      await appWindow.close();
    }
  } catch (err) {
    console.error(err);
  }
}

function onDragCancel() {
  if (!drag) return;
  const d = drag;
  drag = null;
  // No `pointerup` follows a cancel from `onDragMove`, so nothing else would
  // hand the mouse back.
  try {
    els.bar.releasePointerCapture(d.pointerId);
  } catch {
    /* capture already gone */
  }
  hideGhost();
  paintDrag();
  if (d.detached) d.ipc = d.ipc.then(() => invoke("drag_cancel")).catch(() => {});
  // A cancelled drag should leave nothing behind, and dragging through the
  // strip has already moved the tab; put it back where it was picked up.
  const from = tabs.findIndex((t) => t.id === d.id);
  if (from >= 0 && d.startIndex >= 0 && from !== d.startIndex) {
    const [moved] = tabs.splice(from, 1);
    tabs.splice(Math.min(d.startIndex, tabs.length), 0, moved);
  }
  renderTabs();
}

function onTabClick(event) {
  const close = event.target.closest(".tab-close");
  if (!close) return;
  const el = close.closest(".tab");
  if (el) closeTab(Number(el.dataset.id)).catch(toast);
}

/**
 * A focused tab's keys, as the WAI-ARIA tabs pattern has them: ←/→ move focus
 * round the strip without switching, Enter or Space switches to the tab, and
 * Delete closes it by the close button's path. Unmodified keys only, so
 * Ctrl+Tab, Ctrl+W and Alt+←/→ still reach `onKeydown`; stopped once handled.
 */
function onTabsKeydown(event) {
  const el = event.target;
  if (!el.classList.contains("tab")) return;
  if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
  const id = Number(el.dataset.id);
  if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
    const all = [...els.tabs.querySelectorAll(".tab")];
    const step = event.key === "ArrowRight" ? 1 : -1;
    const to = all[(all.indexOf(el) + step + all.length) % all.length];
    for (const t of all) t.tabIndex = t === to ? 0 : -1;
    to.focus();
  } else if (event.key === "Enter" || event.key === " ") {
    activateTab(id).then(focusActiveTab).catch(toast);
  } else if (event.key === "Delete") {
    closeTab(id).then(focusActiveTab).catch(toast);
  } else return;
  event.preventDefault();
  event.stopPropagation();
}

/** The strip is rebuilt on every change, so focus has to be put back on it by hand. */
function focusActiveTab() {
  els.tabs.querySelector(".tab.active")?.focus();
}

/* ---------------- themes ---------------- */

/**
 * The themes the picker offers: one per family, in list order. Rust sorts a
 * family's members adjacently, so the first of each is the one to show.
 */
function themeGroups() {
  const seen = new Set();
  return state.themes.filter((t) => !seen.has(t.group) && seen.add(t.group));
}

/** The entry for the theme on screen, or undefined if config names a stem that is gone. */
function currentTheme() {
  return state.themes.find((t) => t.name === state.theme);
}

/**
 * The member of `group` to switch to. Prefers `mode`, so moving between
 * families keeps you on the side you were already on; a family that only has
 * the other side hands that over rather than refusing.
 */
function themeIn(group, mode) {
  const members = state.themes.filter((t) => t.group === group);
  return members.find((t) => t.mode === mode) ?? members[0];
}

async function applyTheme(name) {
  try {
    const css = await invoke("read_theme", { name });
    // Diagrams are drawn in the theme's palette rather than styled by its CSS,
    // so the ones on screen are drawn again — but only when their look changed.
    // The look costs a style recalc, so only with a diagram on screen.
    const drawn = !els.content.hidden && diagramFigures().length > 0;
    const before = drawn && diagramLook().key;
    els.themeStyle.textContent = css;
    // Themes colour the page on body, not in a --ui-* variable; the active tab wears it.
    document.documentElement.style.setProperty("--page-bg", getComputedStyle(document.body).backgroundColor);
    state.theme = name;
    els.picker.value = currentTheme()?.group ?? "";
    updateThemeToggle();
    if (drawn && diagramLook().key !== before) {
      // The overlay's copy is of the old drawing.
      els.diagramDialog.close();
      redrawDiagrams().catch(console.error);
    }
    return true;
  } catch (err) {
    /*
     * A theme that will not load leaves the window wearing base.css alone,
     * which reads as the app being broken rather than as one bad file. Say
     * which file it was and put the default up instead — but do not save the
     * default over the reader's choice, so a theme they are in the middle of
     * editing is tried again next launch rather than quietly given up on.
     */
    toast(String(err));
    if (name !== state.defaultTheme) await applyTheme(state.defaultTheme);
    // Still the reader's pick: the theme watcher retries `state.theme`, so a
    // file being fixed under the editor comes back on its next save.
    state.theme = name;
    return false;
  }
}

async function selectTheme(name) {
  // Only a theme that actually loaded is worth remembering for next launch.
  if (await applyTheme(name)) await invoke("set_theme", { name });
}

async function loadThemeList() {
  state.themes = await invoke("list_themes");
  els.picker.replaceChildren(
    ...themeGroups().map((t) => {
      const opt = document.createElement("option");
      opt.value = t.group;
      opt.textContent = t.group_label;
      return opt;
    }),
  );
  els.picker.value = currentTheme()?.group ?? "";
  updateThemeToggle();
}

/**
 * The bar's light/dark button. The icon is the side being switched *to*, so it
 * reads as an action like every other button up there.
 *
 * Five of the bundled families have only one side, and `aria-disabled` rather
 * than `disabled` is what says so: Chromium delivers no mouse events to a
 * disabled control, so its `title` never appears — and the tooltip is the whole
 * explanation of why the button does nothing.
 */
function updateThemeToggle() {
  const current = currentTheme();
  const dark = current?.mode === "dark";
  // `toggleAttribute`, not `.hidden`: that IDL property belongs to HTMLElement,
  // and these are SVG elements, so assigning to it would set a plain expando
  // and never reach the DOM — leaving whichever icon the markup ships visible
  // for good.
  els.themeSun.toggleAttribute("hidden", !dark);
  els.themeMoon.toggleAttribute("hidden", dark);

  const other = current && themeIn(current.group, dark ? "light" : "dark");
  const off = !current || other === current;
  els.themeToggle.setAttribute("aria-disabled", String(off));

  const label = off
    ? `${current ? current.group_label : "This theme"} has no ${dark ? "light" : "dark"} version`
    : `Switch to ${dark ? "light" : "dark"} theme`;
  els.themeToggle.title = label;
  els.themeToggle.setAttribute("aria-label", label);
}

function toggleThemeMode() {
  const current = currentTheme();
  if (!current) return;
  const other = themeIn(current.group, current.mode === "dark" ? "light" : "dark");
  if (other === current) return; // single-sided family: the button reads as off
  state.themeMode = other.mode;
  selectTheme(other.name).catch(toast);
}

/** F8 walks the families, keeping the light/dark side you chose. */
function cycleTheme(step) {
  const groups = themeGroups();
  if (!groups.length) return;
  const i = groups.findIndex((t) => t.group === currentTheme()?.group);
  const next = groups[(i + step + groups.length) % groups.length];
  selectTheme(themeIn(next.group, state.themeMode).name).catch(toast);
}

/* ---------------- interactions ---------------- */

let pickerOpen = false;

/**
 * One picker at a time. The native dialog does not own the button behind it, so
 * a second click while it is up opens a second dialog; the extra one is answered
 * with a path nobody asked for. Ignoring the click beats queueing it — the user
 * clicking twice wanted one folder, not two.
 */
async function openDialog(options) {
  if (pickerOpen) return null;
  pickerOpen = true;
  try {
    return await openNativeDialog(options);
  } finally {
    pickerOpen = false;
  }
}

/** Where a picker should open. Rust decides — it can check the path is still there. */
async function startDir() {
  return invoke("picker_dir", { dir: activeDir() }).catch(() => "");
}

async function pickFile() {
  const dir = await startDir();
  const picked = await openDialog({
    multiple: false,
    defaultPath: dir || undefined,
    // Windows and GTK show only the first filter until the user changes it,
    // so the first one has to cover everything that opens as a document.
    filters: [
      { name: "Documents", extensions: [...MD_EXTS, ...JSON_EXTS] },
      { name: "Markdown", extensions: MD_EXTS },
      { name: "JSON", extensions: JSON_EXTS },
      { name: "Images", extensions: ["svg", "png", "jpg", "jpeg", "gif", "webp", "avif", "bmp", "ico"] },
      { name: "All files", extensions: ["*"] },
    ],
  });
  return typeof picked === "string" ? picked : null;
}

async function pickFolder() {
  const dir = await startDir();
  const picked = await openDialog({ directory: true, multiple: false, defaultPath: dir || undefined });
  return typeof picked === "string" ? picked : null;
}

/** Ask for a folder and show it in the sidebar. */
async function chooseFolder() {
  const path = await pickFolder();
  if (path) await openFolder(path, { picked: true });
}

/** The active document's folder, or "" when there is none to name. */
function activeDir() {
  const tab = activeTab();
  if (!tab) return "";
  // Before the first load finishes (or after a failed one) `dir` is empty.
  return tab.dir || dirName(tab.path);
}

/**
 * Toggle the sidebar. Opening shows the folder `folderFor` names — the one a
 * tab switch would — and asks only when there is none to name.
 */
async function showFolder() {
  if (state.folder !== null) {
    closeFolder();
    return;
  }
  const tab = activeTab();
  const named = (tab && folderFor(tab)) || activeDir();
  const path = named || (await pickFolder());
  if (!path) return;
  // What the tab remembers is only about its own folder, not a stand-in for it.
  const own = !!tab && path === tab.folder;
  await openFolder(
    path,
    // No filter to hand back: closing the sidebar blanked every tab's.
    own ? { keep: tab.expanded, picked: tab.picked } : { picked: !named },
  );
}

/**
 * The webview is the whole app: letting it navigate away would leave a dead
 * window. Intercept every link — follow anchors, open sibling documents in
 * place, hand everything else to the system browser.
 */
function onLinkClick(event) {
  // Any namespace: a diagram's `click` links are SVG `<a xlink:href>`.
  const a = event.target.closest("a[*|href]");
  if (!a) {
    // A click focuses a diagram as well, now it can take focus, and a later
    // key — the Escape that closes its overlay or an actor's menu — would
    // turn that into a keyboard focus, so a Space meant to scroll would open
    // it. Let go of it. Not for a key press, which `el.click()` sends with no
    // detail.
    if (event.detail > 0 && diagramBlocks.has(document.activeElement)) document.activeElement.blur();
    // A picture in a document is held to the column width, which is no width at
    // all for a wide diagram. Clicking one opens it where it can be read.
    // Only outside a link: a linked image still means the link.
    const img = event.target.closest("img[data-file]");
    if (img) {
      event.preventDefault();
      openTab(img.dataset.file).catch(console.error);
    }
    // Inside an open menu but not on one of its links: nothing to do.
    if (event.target.closest('g.actorPopupMenu[id$="_popup"]')) return;
    const fig = figureOf(event.target);
    const menu = fig && actorMenu(fig, event.target);
    closeActorMenus(menu);
    if (menu) {
      if (menu.getAttribute("display") === "block") {
        menu.setAttribute("display", "none");
        menu.removeAttribute("transform");
        return;
      }
      // mermaid hangs every menu under the actor's top box, which on a
      // diagram taller than the window is out of sight from the bottom box or
      // far down the lifeline. So it opens where it was clicked, in the
      // drawing's units: just below the click, or just above it where it
      // would run past the drawing's bottom and be cut off. The gap keeps the
      // clicked point clear, so a second click there closes it again. A click
      // on the top box, above where mermaid put it, leaves it there.
      const GAP = 4;
      const panel = menu.querySelector(".actorPopupMenuPanel");
      const top = panel.y.baseVal.value;
      const height = panel.height.baseVal.value;
      const svg = menu.ownerSVGElement;
      const { y } = new DOMPoint(event.clientX, event.clientY).matrixTransform(svg.getScreenCTM().inverse());
      const box = svg.viewBox.baseVal;
      let dy = Math.max(0, y + GAP - top);
      if (top + dy + height > box.y + box.height) dy = y - GAP - height - top;
      if (dy) menu.setAttribute("transform", `translate(0, ${dy})`);
      menu.setAttribute("display", "block");
      return;
    }
    if (fig) openDiagram(fig);
    return;
  }
  closeActorMenus(); // a link is being followed, a menu's own or not
  const href = a.getAttribute("href") ?? a.getAttributeNS("http://www.w3.org/1999/xlink", "href");
  // Nothing to follow: an emptied link — comrak writes `href=""` for a
  // `file:`, `javascript:` or `data:` link, and mermaid `about:blank` for an
  // unsafe `click` URL — or a page that is not the active entry's, a switch
  // still under way. Either way the webview must not follow it on its own:
  // following `""` reloads the page, and every tab in the window goes with
  // it. A refresh of the page on screen — a save, F5 — is not a switch: its
  // links act on it, and one that navigates outdates the refresh.
  const pending = shownToken !== renderToken;
  if (!href || href === "about:blank" || (pending && shownEntry !== currentEntry(activeTab()))) {
    event.preventDefault();
    return;
  }

  // In-page anchor: recorded, then jumped to here rather than by the browser,
  // which would go to the first element of that name — and the app's own come
  // before the document's. Except `#` and `#top` naming nothing in the
  // document: those are "back to top", which the browser does itself.
  if (href.startsWith("#")) {
    const id = decodeId(href.slice(1));
    if (!docTarget(id) && /^(top)?$/i.test(id)) return;
    event.preventDefault();
    pushAnchorEntry(id);
    jumpToAnchor(id);
    return;
  }

  event.preventDefault();

  if (isLocal(href)) {
    const [pathPart, ...rest] = href.split("#");
    const target = resolvePath(activeTab()?.dir ?? "", pathPart, activeTab()?.repo);
    if (DOC_LINK.test(pathPart)) {
      // `notes.md#fc-29` is one link, not two: the file to load and the place
      // in it to land. Dropping the fragment would open every cross-file
      // reference at the top of its document.
      loadPath(target, rest.join("#"));
    } else if (isImage(pathPart)) {
      // A new tab, not this one: the document that linked the diagram is the
      // thing you were reading, and closing the tab is how you get back to it.
      openTab(target).catch(console.error);
    } else {
      // Neither a document nor a picture, so there is nothing to show. Reveal
      // it in the file manager rather than launching it: the link comes from a
      // file the user did not write, and `[setup](../tools/setup.bat)` must not
      // run on a click.
      invoke("reveal_path", { path: target }).catch(toast);
    }
    return;
  }

  openUrl(href).catch(toast);
}

/**
 * Ticks go one at a time, each against the stamp the last one left: two quick
 * ones must not both claim the render's. A tick the file has moved on from is
 * refused; the page then shows the file as it is — on a share that sends no
 * change notices nothing else would.
 */
let ticking = Promise.resolve();

function onTaskToggle(event) {
  const box = event.target;
  const li = box.closest("li[data-sourcepos]");
  if (box.type !== "checkbox" || !li) return;
  const line = Number(li.dataset.sourcepos.split(":")[0]);
  // The tab's entry moves on as soon as a navigation starts, before the new
  // page is on screen; the line number belongs to the document still in the
  // DOM, so take the path — and the stamp — from there too.
  const path = els.content.dataset.path;
  const checked = box.checked;
  ticking = ticking.then(async () => {
    // Re-rendered while it waited: the new page already shows the file as it is.
    if (!box.isConnected) return toast("The file changed; tick it again.");
    try {
      const next = await invoke("toggle_task", { path, line, checked, stamp: els.content.dataset.stamp });
      if (box.isConnected) els.content.dataset.stamp = next;
    } catch (err) {
      toast(err);
      if (box.isConnected) {
        box.checked = !checked;
        refresh().catch(console.error);
      }
    }
  });
}

/**
 * Copy a heading's own Markdown. The backend slices it out of the file rather
 * than the DOM, so what lands on the clipboard is the source, not a round-trip
 * through HTML. The tick is only feedback; it clears itself.
 */
function onCopySection(event) {
  const btn = event.target.closest("button.copy-section");
  if (!btn) return;
  // A second click while the first is still in flight would parse the file
  // again and race the tick.
  if (btn.disabled) return;
  btn.disabled = true;
  const heading = btn.closest("h1, h2, h3, h4, h5, h6");
  const line = Number(heading.dataset.sourcepos.split(":")[0]);
  // As in `onTaskToggle`: the line belongs to the document in the DOM, so the
  // path has to come from there too, not from a tab entry that may have moved on.
  const path = els.content.dataset.path;
  const md = invoke("section_source", { path, line, stamp: els.content.dataset.stamp });
  // WebKit only honours a clipboard write started inside the click itself, so
  // the write begins now and the text lands when the backend has it.
  const item = new ClipboardItem({
    "text/plain": md.then((text) => new Blob([text], { type: "text/plain" })),
  });
  navigator.clipboard
    .write([item])
    .then(() => {
      btn.classList.add("copied");
      setTimeout(() => btn.classList.remove("copied"), 1200);
    })
    // A backend refusal surfaces through the write as a generic clipboard
    // error; the message worth showing is the backend's own.
    .catch((err) =>
      md.then(
        () => toast(err),
        (e) => {
          toast(e);
          refresh().catch(console.error);
        },
      ),
    )
    .finally(() => (btn.disabled = false));
}

/**
 * The two controls a rendered JSON document carries. Rust emits both as plain
 * buttons inside the `<pre>`, so one delegated handler covers every one of them,
 * including those spliced in later by a `more`.
 */
function onJsonClick(event) {
  const fold = event.target.closest("button.fold");
  if (fold) {
    const open = fold.getAttribute("aria-expanded") === "true";
    fold.setAttribute("aria-expanded", String(!open));
    // The body is the button's next sibling but one: the opening bracket sits
    // between them, and the closing one after, which is what the `…` hangs off.
    const body = fold.nextElementSibling?.nextElementSibling;
    if (body) body.hidden = open;
    return;
  }

  const btn = event.target.closest("button.more");
  if (!btn) return;
  // As in `onCopySection`: a second click while the first is in flight would
  // splice the same region in twice.
  if (btn.disabled) return;
  btn.disabled = true;
  const [start, end] = btn.dataset.range.split(":").map(Number);
  // As in `onTaskToggle`: the range belongs to the document in the DOM, so the
  // path has to come from there too, not from a tab entry that may have moved on.
  const path = els.content.dataset.path;
  // Whose document this is, taken now: by the time the chunk is back, another
  // tab may be showing the same file. The entry on screen, not the active one:
  // mid-switch the button is still on the page being left.
  const owner = shownEntry;
  invoke("json_region", { path, start, end })
    .then((html) => {
      // Re-rendered while the fetch was out: this button is no longer in the
      // page, and the new render has one of its own.
      if (!btn.isConnected) return;
      // The chunk goes where the button was, and may itself end in another one.
      btn.insertAdjacentHTML("beforebegin", html);
      btn.remove();
      // How far the document is now loaded, for the next render of it. The
      // innermost button left always has the smallest start, so everything
      // before it is here; with none left the whole document is. A re-render
      // can only restore a prefix, which is exactly what a budget is.
      // With none left the whole document is loaded, whichever button was
      // clicked last, and `render_within` clamps the extent to `MAX_EXTENT`.
      // Never `Infinity`: `JSON.stringify` turns it into `null` — extent 0,
      // chunk one. `reduce`, not a spread: a deeply nested cut leaves one
      // button per open container.
      const loaded = [...els.content.querySelectorAll("button.more")]
        .map((b) => Number(b.dataset.range.split(":")[0]))
        .reduce((a, b) => Math.min(a, b), Number.MAX_SAFE_INTEGER);
      // Only if this is still the document on screen: a navigation during
      // the fetch would otherwise stamp the count on whatever replaced it.
      if (owner && owner === shownEntry && els.content.dataset.path === path)
        owner.extent = loaded;
    })
    .catch((err) => {
      // The file has changed under the range — say so in place rather than in a
      // toast that would be gone before the watcher's re-render lands.
      btn.textContent = String(err);
      btn.disabled = false;
    });
}

/*
 * Drag-to-pan, for the picture and the diagram overlay alike. Pointer capture
 * rather than a document-level listener so a drag that leaves the window
 * still steers the scroll, and so releasing outside it still ends cleanly.
 * `active` says whether there is anything to pan; `moved` hears of a drag
 * that moved, once it ends.
 */
function dragPan(view, active, moved) {
  let pan = null;
  function up(event) {
    if (!pan || event.pointerId !== pan.pointerId) return;
    const didMove = pan.moved;
    pan = null;
    try {
      view.releasePointerCapture(event.pointerId);
    } catch {
      /* capture already gone */
    }
    view.classList.remove("panning");
    if (didMove) moved?.();
  }
  return {
    down(event) {
      if (!active() || event.button !== 0) return;
      // Also what stops the browser starting a drag of its own instead.
      event.preventDefault();
      pan = { pointerId: event.pointerId, x: event.clientX, y: event.clientY, moved: false };
      view.setPointerCapture(event.pointerId);
    },
    move(event) {
      if (!pan || event.pointerId !== pan.pointerId) return;
      if (event.buttons === 0) return up(event);
      const dx = event.clientX - pan.x;
      const dy = event.clientY - pan.y;
      if (!pan.moved && Math.hypot(dx, dy) < PAN_THRESHOLD) return;
      pan.moved = true;
      pan.x = event.clientX;
      pan.y = event.clientY;
      view.classList.add("panning");
      // Dragging the picture or the diagram left means looking further right.
      view.scrollLeft -= dx;
      view.scrollTop -= dy;
    },
    up,
    /** Let go of a drag no pointerup is coming for. */
    cancel() {
      if (pan) {
        try {
          view.releasePointerCapture(pan.pointerId);
        } catch {
          /* capture already gone */
        }
      }
      pan = null;
      view.classList.remove("panning");
    },
  };
}

const imagePan = dragPan(els.imageView, () => picture, rememberImage);

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
  // Any keystroke but a bare modifier dismisses the tree's menu. It floats over
  // the document rather than inside the tree, so a shortcut that hides the
  // sidebar would otherwise leave it pointing at a row that is no longer there.
  // Not the keys the menu itself is listening for, though: dismissing on Enter
  // would take the row away before the item it activates could read it, and
  // `onMenuKeydown` keeps the arrows. Tab does dismiss: focus goes back to the
  // row first, so the browser's own move carries on from there.
  const forMenu =
    els.treeMenu.contains(document.activeElement) && ["Enter", " "].includes(event.key);
  if (
    !els.treeMenu.hidden &&
    !forMenu &&
    !["Shift", "Control", "Alt", "Meta"].includes(event.key)
  ) {
    const li = menuRow?.parentElement;
    showTreeMenu(null);
    li?.focus();
    if (event.key === "Escape") return; // dismissing it was the whole instruction
  }

  // The sidebar's sort menu goes the same way, back to the button it came from.
  const forSortMenu =
    els.sortMenu.contains(document.activeElement) && ["Enter", " "].includes(event.key);
  if (
    !els.sortMenu.hidden &&
    !forSortMenu &&
    !["Shift", "Control", "Alt", "Meta"].includes(event.key)
  ) {
    showSortMenu(false);
    els.sidebarSort.focus();
    if (event.key === "Escape") return;
  }

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
   * here too; both suppress the default.
   *
   * Handled before the modal guard, and with any modifier, so no spelling of
   * a reload key can get past it — Ctrl+F5 and Shift+F5 included.
   */
  const ctrl = event.ctrlKey || event.metaKey;
  // `keyCode` too: WebView2's accelerator reads the virtual key, so Ctrl+К on
  // a Cyrillic layout reloads although `key` is "к". Not `code`: on Dvorak
  // `KeyR` is where P is.
  if (
    event.key === "F5" ||
    event.key === "BrowserRefresh" ||
    (ctrl && !event.altKey && (event.key.toLowerCase() === "r" || event.keyCode === 82))
  ) {
    event.preventDefault();
    refresh();
    return;
  }

  // Otherwise a dialog is modal: let it own the keyboard, Escape included —
  // but not WebView2's Back and Forward, which would walk the `#id` history
  // under it.
  if (els.settings.open || els.updateDialog.open || els.diagramDialog.open) {
    if (event.altKey && (event.key === "ArrowLeft" || event.key === "ArrowRight")) event.preventDefault();
    return;
  }

  // Tab closes it too, from its button, for the reason the tree's menu does.
  if ((event.key === "Escape" || event.key === "Tab") && !els.openMenu.hidden) {
    showOpenMenu(false);
    els.openMore.focus();
    if (event.key === "Escape") return;
  }
  if (event.key === "Escape" && closeActorMenus()) return;

  // A diagram or picture with focus opens as a click on it would. Space
  // too, which would otherwise scroll the page. Not on a held key's repeat,
  // which would open a picture's tab again before the first has taken the
  // page away.
  if ((event.key === "Enter" || event.key === " ") && !ctrl && !event.altKey && !event.shiftKey && !event.repeat) {
    const el = document.activeElement;
    // Only a keyboard focus. A mouse press lets go of any focus it gave (the
    // mousedown listener, and `onLinkClick`); this is the last guard.
    if (
      el &&
      els.content.contains(el) &&
      el.matches(":focus-visible") &&
      el.matches(OPENABLE)
    ) {
      event.preventDefault();
      if (el.tagName === "IMG") pictureBack = { path: els.content.dataset.path, mark: markOpenable(el) };
      el.click();
      return;
    }
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

  // Windows reports AltGr as Ctrl+Alt, and on a German or Nordic keyboard
  // AltGr is how `[` and `]` are typed. Nothing below is a Ctrl+Alt shortcut.
  if (!ctrl || event.altKey) return;

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
  } else if (key === "f" && event.shiftKey) {
    event.preventDefault();
    if (state.folder === null) await showFolder();
    if (state.folder === null) return; // the picker was cancelled
    els.treeFilter.focus();
    els.treeFilter.select();
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
  } else if (key === ",") {
    // Cmd+, is the Mac idiom for preferences, and the editors copied it onto
    // the other two platforms. Same door as the gear.
    event.preventDefault();
    showOpenMenu(false);
    els.settings.showModal();
  }
}

/** Thumb buttons on a mouse, as in a browser. */
function onMouseUp(event) {
  // Over a zoomed diagram, the thumb's Back closes it rather than paging the
  // document behind; Forward has nowhere to go. Neither is left to the
  // webview, which would walk the `#id` history under it.
  if (els.diagramDialog.open) {
    if (event.button === 3 || event.button === 4) event.preventDefault();
    if (event.button === 3) {
      // A mouse close, as the toolbar's × by a click: let go of the way back
      // and of the diagram the dialog hands focus back to.
      zoomed.back = null;
      els.diagramDialog.close();
      if (diagramBlocks.has(document.activeElement)) document.activeElement.blur();
    }
    return;
  }
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
  els.emptyFolderBtn.addEventListener("click", chooseFolder);
  els.emptyReopenBtn.addEventListener("click", () => reopenSession().catch(toast));

  els.folderBtn.addEventListener("click", async () => {
    showOpenMenu(false);
    await showFolder();
  });
  els.sidebarClose.addEventListener("click", closeFolder);
  els.sidebarName.addEventListener("click", chooseFolder);
  els.sidebarSort.addEventListener("click", () => showSortMenu());
  els.sortMenu.addEventListener("click", (e) => {
    const item = e.target.closest("button[data-sort]");
    if (!item) return;
    showSortMenu(false);
    els.sidebarSort.focus(); // hiding the focused item would drop it to the body
    setFolderSort(item.dataset.sort).catch(toast);
  });
  els.treeFilter.addEventListener("input", () => {
    rememberFilter(els.treeFilter.value);
    applyTreeFilter();
  });
  els.treeFilter.addEventListener("keydown", async (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation(); // onKeydown would read it as "close the open menu"
      els.treeFilter.value = "";
      rememberFilter("");
      applyTreeFilter();
      els.treeFilter.blur();
    } else if (event.key === "Enter" && els.treeFilter.value.trim()) {
      event.preventDefault();
      const row = [...els.tree.querySelectorAll(".tree-row[data-path]:not([data-dir])")].find(
        (r) => !r.closest("[hidden]"),
      );
      if (row) await loadPath(row.dataset.path);
    }
  });
  // Toast, not the console: Shift+click asks Rust for a window, and a failure
  // there has nothing of its own to show — the other two branches render their
  // own error page.
  els.tree.addEventListener("click", (e) => onTreeClick(e).catch(toast));
  els.tree.addEventListener("auxclick", (e) => onTreeAuxClick(e).catch(console.error));
  els.tree.addEventListener("pointerdown", (e) => {
    // Autoscroll is the middle button's default action, and it would swallow
    // the auxclick that opens the tab. Same trick as the tab strip.
    if (e.button === 1) e.preventDefault();
  });
  els.tree.addEventListener("contextmenu", onTreeContextMenu);
  els.tree.addEventListener("keydown", onTreeKeydown);
  // Roving tabindex: the tab stop goes wherever focus lands, by key or by click.
  els.tree.addEventListener("focusin", setTreeStop);
  els.treeMenu.addEventListener("click", (e) => onTreeMenuClick(e).catch(toast));
  for (const menu of [els.openMenu, els.sortMenu, els.treeMenu]) menu.addEventListener("keydown", onMenuKeydown);
  // The tree answers metaKey as well as ctrlKey; on the Mac only one of those
  // is the one people reach for, and Ctrl+click is the right-click that opened
  // this menu in the first place.
  if (isMac)
    for (const key of els.treeMenu.querySelectorAll(".menu-key"))
      key.textContent = key.textContent.replace("Ctrl", "⌘");

  els.openMore.addEventListener("click", () => showOpenMenu());
  els.openMenu.addEventListener("click", async (e) => {
    const item = e.target.closest("button[data-mode]");
    if (!item) return;
    showOpenMenu(false);
    els.openMore.focus(); // hiding the focused item would drop it to the body
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
      // Not the menu itself: hiding it here would leave its own click with
      // nothing to land on.
      if (!els.treeMenu.hidden && !e.target.closest("#tree-menu")) showTreeMenu(null);
      // Nor the button that opened it, which would otherwise close here and
      // reopen on the click that follows — same reason as `#open-split`.
      if (!els.sortMenu.hidden && !e.target.closest("#sort-menu, #sidebar-sort"))
        showSortMenu(false);
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
  for (const radio of els.diagramColourRadios) {
    radio.addEventListener("change", () => {
      if (radio.checked) {
        showDiagramColours(radio.value);
        invoke("set_diagram_colours", { colours: radio.value }).catch(console.error);
      }
    });
  }
  for (const radio of els.reopenRadios) {
    radio.addEventListener("change", () => {
      if (!radio.checked) return;
      // "Start fresh" throws the saved session away there and then, so an
      // offer still standing behind the dialog is now for nothing.
      if (radio.value === "off") els.emptyReopenBtn.hidden = true;
      invoke("set_reopen", { mode: radio.value }).catch(console.error);
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
  // Settings is closed first so the two dialogs never stack; Later then
  // returns to the document, same as coming from the bar.
  els.settingsUpdate.addEventListener("click", () => {
    els.settings.close();
    showUpdateDialog();
  });
  els.updateNow.addEventListener("click", runUpdate);
  els.updateNotesBtn.addEventListener("click", () => {
    if (state.update) openUrl(state.update.release_url).catch(toast);
  });

  // The picker's value is a family, not a theme: keep the side you chose.
  els.picker.addEventListener("change", (e) => {
    selectTheme(themeIn(e.target.value, state.themeMode).name).catch(toast);
  });
  els.themeToggle.addEventListener("click", toggleThemeMode);
  els.content.addEventListener("click", onCopySection);
  els.content.addEventListener("click", onLinkClick);
  // A press that never becomes a click on the page — a right or middle
  // button, or one dragged out of it — focuses a diagram or picture too, and
  // any key after makes that a keyboard focus. Let go of it once the press
  // has done its focusing. `onLinkClick` lets go at the click as well, which
  // a fast click can reach before this does.
  els.content.addEventListener("mousedown", () =>
    setTimeout(() => {
      const el = document.activeElement;
      if (els.content.contains(el) && el.matches(OPENABLE)) el.blur();
    }),
  );
  els.content.addEventListener("click", onJsonClick);
  els.content.addEventListener("change", onTaskToggle);
  // A form on the page can only come from a diagram — comrak strips raw
  // HTML — and submitting it, by a button or Enter, would navigate the webview
  // and reload the viewer. Settings and Update have their own, outside both.
  els.content.addEventListener("submit", (e) => e.preventDefault());
  els.diagramView.addEventListener("submit", (e) => e.preventDefault());
  els.toast.addEventListener("click", () => (els.toast.hidden = true));
  els.back.addEventListener("click", () => go(-1));
  els.forward.addEventListener("click", () => go(1));

  // Not passive: preventDefault is what keeps a Ctrl+wheel zoom from scrolling
  // the box at the same time, and a passive listener is not allowed to.
  els.imageView.addEventListener("wheel", onImageWheel, { passive: false });
  els.imageView.addEventListener("pointerdown", imagePan.down);
  els.imageView.addEventListener("pointermove", imagePan.move);
  els.imageView.addEventListener("pointerup", imagePan.up);
  els.imageView.addEventListener("pointercancel", imagePan.up);
  els.imageView.addEventListener("dblclick", onImageDblClick);
  els.imageTools.addEventListener("click", onZoomClick);
  els.diagramView.addEventListener("wheel", onDiagramWheel, { passive: false });
  els.diagramView.addEventListener("pointerdown", diagramPan.down);
  els.diagramView.addEventListener("pointermove", diagramPan.move);
  els.diagramView.addEventListener("pointerup", diagramPan.up);
  els.diagramView.addEventListener("pointercancel", diagramPan.up);
  els.diagramView.addEventListener("lostpointercapture", (e) => e.target === els.diagramView && diagramPan.up(e));
  els.diagramView.addEventListener("dblclick", onDiagramDblClick);
  // The toolbar is no scroll box, so a wheel over it would scroll the document behind.
  els.diagramDialog.addEventListener("wheel", (e) => !els.diagramView.contains(e.target) && e.preventDefault(), {
    passive: false,
  });
  // A drag that ends on a diagram's link must not follow it; the page behind is where links are followed.
  els.diagramView.addEventListener("click", (e) => e.target.closest("a") && e.preventDefault());
  els.diagramTools.addEventListener("click", onDiagramToolsClick);
  els.diagramDialog.addEventListener("keydown", onDiagramKeydown);
  els.diagramDialog.addEventListener("close", onDiagramClose);
  window.addEventListener("resize", onResize);

  els.tabs.addEventListener("pointerdown", onTabPointerDown);
  els.docName.addEventListener("pointerdown", onDocNamePointerDown);
  // Capture lands on the bar, so the whole drag is tracked from there.
  els.bar.addEventListener("pointermove", onDragMove);
  els.bar.addEventListener("pointerup", onDragEnd);
  els.bar.addEventListener("pointercancel", onDragCancel);
  // Only the element that holds the capture: a touch captures the tab or the
  // picture itself first, and handing that over to the bar fires this on the
  // child — which bubbles here and would cancel every touch drag.
  els.bar.addEventListener("lostpointercapture", (e) => e.target === els.bar && onDragCancel());
  els.imageView.addEventListener("lostpointercapture", (e) => e.target === els.imageView && imagePan.up(e));
  els.tabs.addEventListener("click", onTabClick);
  els.tabs.addEventListener("keydown", onTabsKeydown);

  // onKeydown is async, so a failed open or close comes back as a rejected
  // promise no listener would ever look at: a shortcut would just do nothing.
  document.addEventListener("keydown", (e) => onKeydown(e).catch(toast));
  document.addEventListener("mouseup", onMouseUp);
  // The one change `updateChrome` never hears about, and the reading position
  // is otherwise only banked on the way out of a tab — so quitting after a
  // scroll would come back at the top of the document. Recording it here and
  // not in the report itself is deliberate: a render puts the page back at the
  // top before restoring the saved offset, and a timer firing in that gap
  // would write the zero over the position it is about to restore. On a real
  // scroll the current offset is by definition the right answer, and the
  // restore fires one of its own, so the last write is the correct one.
  // `rememberScroll` banks on the entry whose page is on screen, so scrolling
  // the old document while a switch loads stays on the old document's entry.
  window.addEventListener(
    "scroll",
    () => {
      rememberScroll();
      scheduleReport();
    },
    { passive: true },
  );
  // A picture pans in its own box, by wheel, scrollbar and keys as well as by
  // drag; the same holds for it.
  els.imageView.addEventListener(
    "scroll",
    () => {
      rememberImage();
      scheduleReport();
    },
    { passive: true },
  );
  // A theme can follow the OS — `color-scheme: light dark` over a see-through
  // page, or rules of its own under `prefers-color-scheme` — and then a flip
  // changes the page's side with no theme change to redraw its diagrams, or to
  // recolour what wears the page colour. `redrawDiagrams` skips any already
  // drawn in the look the page now has. `--page-bg` is refreshed only once a
  // theme has set it; until then the active tab and the overlay use fallbacks
  // of their own.
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    const root = document.documentElement.style;
    if (root.getPropertyValue("--page-bg")) root.setProperty("--page-bg", getComputedStyle(document.body).backgroundColor);
    // The overlay's backdrop has flipped already, and its copy is of the old
    // drawing: close it now, as a theme change does, not once the redraw lands.
    if (zoomed && diagramLook().key !== zoomed.look) els.diagramDialog.close();
    if (!els.content.hidden && diagramFigures().length > 0) redrawDiagrams().catch(console.error);
  });
  // Chromium fires auxclick for the thumb buttons too; swallow it so the
  // default "navigate" behaviour cannot fight our own handling.
  document.addEventListener("auxclick", (e) => {
    if (e.button === 3 || e.button === 4) e.preventDefault();
  });

  const settings = await invoke("get_settings");
  state.crossWindowDrag = settings.cross_window_drag === true;
  state.caseInsensitivePaths = settings.case_insensitive_paths === true;
  state.defaultTheme = settings.default_theme;
  els.autoUpdate.checked = settings.auto_update_check !== false;
  els.appVersion.textContent = settings.version ?? "";
  // Before anything can open a folder, so the first listing is already in the
  // order the reader last chose rather than sorted twice.
  // The file is the user's to edit, and the menu ticks one of exactly two rows.
  state.folderSort = settings.folder_sort === "modified" ? "modified" : "name";
  showOpenMode(settings.open_mode ?? "tab");
  showReopen(settings.reopen ?? "ask");
  showDiagramColours(settings.diagram_colours === "mermaid" ? "mermaid" : "theme");
  await loadThemeList();
  await applyTheme(settings.theme);
  // The saved theme is the side the reader last chose, so start from it.
  state.themeMode = currentTheme()?.mode ?? state.themeMode;

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
  await listen("diagram-colours-changed", (e) => showDiagramColours(e.payload));
  // Broadcast on purpose: one install is happening to the whole app, so every
  // window's dialog should count along with it.
  await listen("update-progress", (e) => {
    setInstalling(true); // another window's install is this window's too
    // A dialog already open when another window's install began shows no
    // progress yet, and may still show an earlier failure's reason.
    els.updateProgress.hidden = false;
    els.updateError.hidden = true;
    showUpdateProgress(e.payload);
  });
  // Every window hears of a failure here, and gets its Update button back.
  await listen("update-failed", () => {
    setInstalling(false);
    els.updateProgress.hidden = true;
    els.updateProgress.textContent = "";
    // The window that asked has its own reason from `invoke`, which may have
    // landed first; every other window says only that it failed.
    if (els.updateError.hidden) {
      els.updateError.textContent = "The update failed.";
      els.updateError.hidden = false;
    }
  });
  // Rust asks once the update is downloaded and waits for the answer. A scroll
  // is reported only once it has settled for `REPORT_DELAY`, and the restart
  // will not wait for that, so bank and report the reader's place now.
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
    // Rust clears the drag before it hands the tab over, so the caret is
    // already gone and cannot say where this lands. The slot comes from where
    // the tab was dropped, the same way the caret was placed while it hovered
    // — measured with the caret taken out, so it does not shift the tabs.
    setCaret(-1);
    await adoptTab(e.payload.tab, tabs.length > 1 ? insertionIndex(e.payload.x) : tabs.length);
  });
  // The window a torn-off tab was on its way to never opened, so the tab comes
  // back rather than disappearing with it.
  await listenHere("tab-spawn-failed", async (e) => {
    await adoptTab(e.payload.tab, tornOffIndex);
    toast("That tab could not be given a window of its own.");
  });

  // Whatever this window was created to show: a file-association open, a tab
  // torn off another window, or everything it had before an update restart.
  const pending = await invoke("take_pending");
  if (pending?.kind === "path") {
    await openTab(pending.path);
  } else if (pending?.kind === "tab") {
    await adoptTab(pending.tab, 0);
  } else if (pending?.kind === "session") {
    await restoreTabs(pending.tabs, pending.active, pending.sidebar);
  } else if (!tabs.length) {
    // A file that arrived and rendered first must not be hidden behind the
    // empty screen.
    show("empty");
    updateChrome();
    // Nothing to open, but something waiting to be: the empty screen grows a
    // button offering the session the last run left behind.
    if (pending?.kind === "offer") showOffer(pending.windows, pending.tabs);
  }

  // Window starts hidden so the first frame is already themed and painted. A
  // restored maximize waits for the same reason: on Windows it shows the window.
  if (pending?.maximized) await appWindow.maximize().catch(() => {});
  await appWindow.show();
  // Showing does not raise a window whose process is in the background, and a
  // window created for a file the user just opened has to land in front. One
  // restored *beside* such a file says so in `behind`, and raises that window
  // instead of itself — whichever of the two finishes booting last, the file
  // the reader asked for ends up on top.
  const front = pending?.behind
    ? await window.__TAURI__.window.Window.getByLabel(pending.behind)
    : null;
  await (front ?? appWindow).setFocus().catch(() => {});

  // Where the window stands, and which one was in front, are saved with the
  // tabs, and nothing else notices them changing. Maximize arrives as a
  // resize; so does minimize, and Rust answers that one with the frame it last
  // had. Registered only now: a report fired while the tabs are still being
  // restored would say this window has none, and `Frame::apply` resizes it
  // before they are.
  window.addEventListener("resize", scheduleReport);
  window.addEventListener("focus", scheduleReport);
  appWindow.onMoved(scheduleReport).catch(console.error);
  // A close waits for this window's last report. Anything still inside
  // REPORT_DELAY — a scroll, a move, a tab change hard on another — would
  // otherwise go with the window. Tauri holds the close while a listener is
  // registered and destroys the window once it resolves; Rust closes it
  // anyway if the page cannot answer (`CLOSE_WAIT`). Registered with the
  // others, after boot: a window still restoring has nothing to add.
  appWindow.onCloseRequested(() => reportSession()).catch(console.error);
  // The same last report for the macOS Quit, the only thing that sends this:
  // Rust waits for every booted window's answer before it exits. Registered
  // after boot for the same reason as the close listener, and a window that
  // does not answer in time keeps its last report.
  listen("quit-requested", () => reportSession()).catch(console.error);

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
