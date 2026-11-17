# Text That Stops Shrinking When Zoomed Out Plan

> **Status (2026-11-17): done.** Written, reviewed (five passes: ten findings, then nine,
> five, two and none, all taken) and ruled on the same day: **option A, wait for WebKit**
> (the owner). Nothing was built and nothing was tried. Recorded as an accepted limit with a
> trigger (`open-items.md`); Task 2, the trial of the private function, was not run and
> stands ready if the limit is reopened.

**Spec:** the open item *On macOS, text stops shrinking below about 64% zoom*
(`docs/open-items.md` → Deferred, added 2026-11-17 in the triage of
`archive/plans/smooth-pinch-macos.md`).

**Goal:** the cause and its reach written down, and the owner's ruling on what the app does
about it.

**Architecture:** none until the ruling. Of its four options one changes no code, one is an
estimated 15 to 25 lines of Rust for macOS alone, and two are changes to the zoom itself.

## What the user sees

On macOS and Linux, zooming a document out shrinks the pictures, the spacing and the reading
column, but below a point not the letters: in the default theme body text stays 9 px tall
on screen from 64% down, in most other themes from 56% down. At 50% in the default theme the
letters are 1.29 times what the level says (measured), so a line holds fewer of
them than at 100%, the text wraps differently, and a code block runs out of its box sooner.
By the keys, the wheel, Settings and a pinch alike, since the document zoom shipped
(`e3da32d`, first in 1.9.1). Windows shrinks its text all the way (measured).

## This was known, and kept once

The item this plan is for says the walk of the smooth pinch found the fault, and leaves open
whether Linux and Windows have it. Both are wrong: the record was not searched when the item
was written.

- `archive/plans/document-zoom.md`, Task 1 (2026-11-12): at 50%, "Lorem" is 29 px wide on
  Linux (WebKitGTK 2.52.6) and 28 px on macOS where half size would be 23 and 22, a ratio of
  9/7 on both. Its Task 1b, in a 700 px window: at 50% the column takes its measure of 100
  characters on Windows (WebView2 157) and of 75 on the other two, where the floor makes a
  character wider. That Windows draws its text at half size is taken from that, and from
  Chromium's source (*Found*); its width was not measured.
- `docs/closed-items.md`, *Accepted limits*: **WebKit doesn't draw body text smaller than
  9 px**, kept by the owner on 2026-11-13 with no trigger: "the engine's own floor, and only
  at the two lowest levels".

What has changed since that ruling, and is the case for looking again:

- **A pinch on macOS now stops on any level**, so the floor is met at every level under 64%
  for body text, not at 50% alone; and a document can be held at 55% or 60%.
- **The reach is wider than "the two lowest levels"** once smaller text is counted (the table
  below): footnote marks, and code in a line inside a table or a footnote, meet it at 75%
  and 80%.
- **The cause is traced**, an upstream change is found, and so is a possible way for the app
  to lift the floor on macOS (*Found*).

## Found (read in the engines' source on 2026-11-17 unless marked)

- **The cause.** WebKit will not draw text under a "minimum logical font size", 9 px by
  default (`MinimumLogicalFontSize`, `Source/WTF/Scripts/Preferences/UnifiedWebPreferences.yaml`),
  unless the page asked for less than that in an absolute size. The rule is tested on the
  size before zoom and applied to the size after it (`usedFontSizeFromComputedSize`,
  `Source/WebCore/style/StyleFontSizeFunctions.cpp`): a size of 9 px or more at 100% is
  never drawn under 9 px. The zoom it uses is the element's own (`style.usedZoom()`), which
  is where the `zoom` on `#content` (`src/base.css:1696`) comes in.
- **A stylesheet cannot step round it.** Every size in a document is 9 px or more at 100%,
  whatever its unit, so every one is under the rule. `text-size-adjust` works on iOS only;
  text inside an SVG is exempt, which is why diagrams shrink, but prose cannot be set that
  way. Moving the scale out of `zoom` into the sizes themselves is option C.
- **No public setting lifts it.** `WKPreferences` offers `minimumFontSize` only, the hard
  minimum, 0 by default; nothing for the logical one in `WKPreferences.mm` or
  `WKPreferencesPrivate.h`, and its feature switches are on/off only. WebKit reads two kinds
  of override from a program's defaults (`WebPreferencesCocoa.mm`): debug ones, which take
  true/false and whole numbers, and persistent ones, which take this kind but only for
  preferences made with a name, and a web view's are made with none (`WKPreferences.mm:49`).
  WebKitGTK's `WebKitSettings` has `minimum-font-size` alone.
- **A private function does.** `WKPreferencesSetDoubleValueForKeyForTesting`
  (`Source/WebKit/UIProcess/API/C/WKPreferencesRefPrivate.h:65`, `WKPreferences.cpp:105`)
  sets any number-valued preference by its name and hands the change to every page. On macOS
  the handle it takes is the `WKPreferences` object itself (`Shared/Cocoa/APIObject.mm`, read
  by the review), which Tauri's `with_webview` reaches through the web view's configuration.
  It is on WebKit's branch of the macOS 13 era, the app's oldest system. **Not verified:**
  that the system's WebKit exports it, and that the call takes effect.
- **WebKit has changed the rule.** Commit `006da4a5c5bd` of 2026-08-04 (*moneycontrol.com:
  Overlapping content at zoom level 50%*, bug 319939, `318509@main`) multiplies the floor by
  the zoom when the zoom is under 1, "so small text still shrinks proportionally with the
  page (as other engines do)". That 50% then draws at half size is reasoned from the code:
  the commit's test is of the browser's own page zoom, though the code takes the element's
  zoom too.
- **That change is not on a release branch.** `safari-7625-branch` and its newest
  sub-branches (`.2.8`, `.2.8.18`) left `main` on 2026-07-07 and `webkitglib/2.54` on
  2026-07-22; on each the file's newest change is of 2026-06-04, so it was not picked across
  either. What a macOS release really carries is shown only by measuring it: the owner's
  Mac (macOS 26.7.1) has the floor. Which update will carry the change is not known; on
  Linux it waits for the distribution's WebKitGTK, not before 2.56 (reasoned).
- **Chromium** applies its rule of the same name to the size before the zoom and multiplies
  the zoom in after (`third_party/blink/renderer/core/css/font_size_functions.cc`), which
  agrees with Windows as measured.

**Which text, at which level, in the four Azure DevOps themes**, of which one is the default
(`config.rs:7`). They keep the sizes of `base.css`; the root is 16 px, and the document font
sets the family only. Worked out, and measured for body text and a code block on the Mac. A
size stops shrinking below 9 px over itself.

| Text | Size at 100% | Stops shrinking below | Ladder levels hit |
|---|---|---|---|
| `h1`, `h2` | 28, 21 px | 32%, 43% | none |
| `h3` | 17 px | 53% | 50% |
| `h4` | 15 px | 60% | 50% |
| Body, `h5` | 14 px | 64% (measured) | 50% |
| A table | 13.3 px | 68% | 50%, 67% |
| `h6` | 13 px | 69% | 50%, 67% |
| Code in a line | 12.6 px | 71% | 50%, 67% |
| A code block, footnotes | 12.25 px | 73% (measured for code) | 50%, 67% |
| Code in a line in a table, in an `h6` | 12, 11.7 px | 75%, 77% | 50%, 67%, 75% |
| A footnote's mark (`sup`) | 11.7 px | 77% | 50%, 67%, 75% |
| A diagram's error text | 11.4 px | 79% | 50%, 67%, 75% |
| Code in footnotes, in a line and as a block | 11, 10.7 px | 82%, 84% | 50% to 80% |
| A mark inside the footnotes (rare) | 10.2 px | 88% | 50% to 80% |

A document has no other small text: raw HTML is dropped, so no `sub` or `small` reaches the
page, and the only `sup` is a footnote's mark (`render.rs`). So in these four themes body
text, headings from `h3` down, tables and code blocks are true to the level from 75% up, and
every kind of text only from 90% up. At 67% a code block is 10% too large, code in a line 7%,
a table 1%.

**In the other eleven bundled themes** the fault is smaller (`src-tauri/themes/`, worked
out). Ten of them (the GitHub, Dracula and Solarized families and Sakura) set body text at
16 px and their headings in `em` of it: body text stops shrinking below 56%, a table below
59%, a code block and footnotes below 64%, `h6` below 66%, all of them at the 50% level
alone. At 67% what is at the floor there is a footnote's mark, code inside footnotes, code
in a line in an `h5` or `h6`, and a diagram's error text. Tufte, at 16.8 px, is a little
lower still: body text below 54%, and at 67% only code inside footnotes.

## Tasks

### Task 1: the ruling

Put to the owner now (*The ruling*). Options A, B and C need no measurement first; D needs
Task 2 before it is built.

### Task 2: a trial of the private function, on the Mac (option D only)

On the owner's go, in a debug build on a pushed `walk/*` branch, by script; the trial's code
is not kept. The level is set with the app's own `set_doc_zoom`, and put back to 100% at the
end, since it is saved.

- **T1.** The function resolves by name at run time (`dlsym`) on the owner's Mac. If it does
  not, the trial ends and D is off.
- **T2.** In the default theme, a test document with one line of each kind in the table,
  each `nowrap`, and for each a control: the same text outside the zoom, with the line's own
  computed family, weight and style copied onto it, sized in px at the size times the level
  (a size given in px is drawn as given, under 9 px too). The text itself is measured, with
  a `Range`, not its box: code in a line has padding. At 50%, and at five points under each
  kind's worked-out floor, rounded down, where the floor makes the line 6% wider or more:
  each line's width against its control's, to within 2%. Not at the floor itself, where the
  two differ by under half a percent. `h3`, whose floor is 53%, is tested at 50% alone
  (5.9% wider), and `h1` and `h2` never meet the floor. No level outside 50 to 300 is asked
  for: `set_doc_zoom` turns one into 100 without a word (`config.rs:79-85`). Without the
  call: the floors of the table. With it, set once to 4.5 (9 times the lowest level) when a
  window is made: every kind follows its control down to 50%. A control, not the level,
  because a font's spacing changes with its size.
- **T3.** At 100% nothing moves: the width of every text element of the app's own interface
  and of the README, the same with the call and without.
- **T4.** It holds in a second window, in a torn-off tab's window, and after a reload of
  the web view, by script with `location.reload()` (`F5` re-reads the document and reloads
  no web view, `app.js:5831-5838`). The web view's own menu is native and out of a script's
  reach: if it has a Reload on macOS, which is not known, the owner tries it by hand.
- **T5.** With the function's name misspelt, so that the lookup finds nothing, the app
  starts and behaves as today.
- Not reachable: a Mac on macOS 13 or 14.

### Task 3: the records, by the ruling

- **A:** the item moves from *Deferred* to *Accepted limits* in `open-items.md`, for macOS
  and Linux, with the cause, the table's reach, WebKit's commit and the trigger; the entry
  in `closed-items.md` gains a line pointing to it.
- **D, B or C:** this plan gains the build as new tasks and goes through the plan review
  loop again before anything is built. Whatever stays for Linux is recorded as under A.

## The ruling

**What it is.** On macOS and Linux, zooming a document out below 64% in the default theme,
and below 56% in most others, stops making the letters smaller while the rest of the page
goes on shrinking; smaller text such as footnotes stops sooner. It is the web engine's
rule. The owner kept it as a limit on 2026-11-13, when it was known at 50% only, and
deferred it for a plan on 2026-11-17.

**Why now.** A pinch has since made every level reachable, the cause is traced, and there
are now two ways it can end without the app rebuilding its zoom: the engine's makers changed
the rule in August 2026, and a private function may lift it on macOS today, if Task 2 shows
that it works.

- **A. Wait for WebKit.** No code. It stays as it is until a system update carries WebKit's
  change (macOS or Safari on a Mac, the WebKitGTK package on Linux), then mends itself with
  no release of ours. Recorded as an accepted limit; reopened by measuring one line at 50%
  on the owner's Mac after such an update, or at the next change to the document zoom.
  Costs nothing. Risks: no date is known for either system, and a Mac that stays on an old
  system (the app runs from macOS 13) may never get it.
- **B. Stop zooming out where the text stops.** On macOS and Linux the lowest level becomes
  75%, where body text, headings, tables and code blocks are true in the four Azure DevOps
  themes (the small kinds are not until 90%; in the other eleven themes 67% is already clean
  but for footnotes and a few rare kinds, so a stop at 75% takes a level from them that is
  all but clean). The step is worked out
  in Rust from a fixed ladder, so the stop is a platform switch there or a new argument to
  `step_doc_zoom` and `set_doc_zoom`; the 50% of a pinch's clamp (`app.js:4591`), of its
  toast (`4682`) and of `doc_zoom` and its tests (`config.rs:79-85`, `410-435`) follow, and
  a level saved below the stop needs a rule. An
  estimate of 30 to 50 lines, not counted, and walks on three platforms. It takes 50% and
  67% away from Mac and Linux readers, where they work today in their uneven way.
- **C. Below the floor, scale the page another way.** Lay the page out at a level the text
  follows and shrink the rest with a transform, or move the scale into the font sizes with a
  counter-zoom for everything measured in px. Either is a rebuild of the zoom under 75% on
  WebKit: the page's height, find, links to headings, the place-keeping and the pinch all
  meet something they do not know. Large, unmeasured, and unneeded once WebKit's change
  ships.
- **D. Lift the floor on macOS with WebKit's private function.** When a window is made, set
  the minimum to 4.5 px, so every size in a document follows the level down to 50%. An
  estimated 15 to 25 lines of `unsafe` Rust for macOS alone, not counted, with `objc2`
  (already in the tree through Tauri) made a direct dependency, as `gtk` is for Linux. It
  would mend every Mac where T1 and T2 pass, without waiting for an update; macOS 13 and 14
  cannot be tried, and the function being in WebKit's source of that era does not prove
  their system exports it. It needs no undoing when WebKit's change ships: the two together
  give a floor of 4.5 px times the level, under every size the app can show (reasoned from
  the source). Risks: the function is private and named for tests, so an update of macOS
  may drop or change it. Looked up by name at run time, its loss leaves the app as it is
  today, not broken, though nothing tells us. A function kept under the same name but
  changed would not be caught: other arguments could crash the app at start, another
  meaning for the key would pass unseen (rare, by how Apple treats such functions;
  reasoned). It changes the setting for the whole page, the
  app's own interface with it; no text there is under 9 px at 100% (reasoned; T3 measures).
  It does nothing for Linux, where no such route was found. It is not known to work until
  Task 2 has run.

**What bears on A against D.**

- **How much of the fault the reader meets depends on the theme.** In the default theme and
  its three siblings body text is untrue at every level under 64%; in the other eleven only
  under 56%, which among the fixed levels is 50% alone.
- **A mends only the Macs that get the update.** One that no longer gets Safari's updates
  keeps the floor for good (reasoned; Apple's list of supported systems was not checked).
- **D costs nothing later**, as above, but puts a private function in the app for good
  unless it is taken out again once the systems have caught up.

**Known and not.** Measured: the floor on the owner's Mac and on Linux at 50%, and none on
Windows. Read: the cause, the missing public setting, the private function, WebKit's change
and that the release branches lack it. Worked out: the table, bar two rows. Not known:
whether the private function works on the system's WebKit; when WebKit's change reaches a
Mac or a Linux desktop.

**Ruled 2026-11-17 by the owner: A.** Turned down: trying D first and then ruling, D itself,
and B. C was not offered again.

**Recommendation: A, with D close behind.** Zooming a document out below 64% still works,
only unevenly; the owner kept it once; the engine's makers have since mended it. B removes
levels that work and C rebuilds what shipped two releases ago: neither is worth it. D is
small, and safe if the function is ever removed though not if it is changed under the same
name; it is the one to take if the owner wants it mended on the Mac now and
not at Apple's pace; what counts against it is a function meant for WebKit's own tests,
kept in the app for a gap that closes by itself on every Mac still updated. Task 2 costs one
short session on the Mac and says whether D can be had at all, so "try D first, then rule"
is a fair answer too.

## Limits known now

- WebKit's and Chromium's source were read at their `main` of 2026-11-17, not at the versions
  the app runs on; the floors themselves are measured.
- Under every option but C, Linux keeps the floor until its WebKitGTK carries the change.
- The table is for the four Azure DevOps themes and the paragraph under it for the other
  eleven; a reader's own theme may set other sizes.

## Chosen without asking

- **The README stays as it is** under option A. It says a document zooms from 50% to 300%,
  which holds; a line about an engine's rule that is on its way out would outlive it.
- **No measurement on Windows or Linux.** Both were measured at 50% in the document zoom's
  trial, and nothing in the options turns on a finer figure there.
- **The table is not measured row by row** unless D is tried, whose T2 does it.

## Review

**Pass 1, ten findings, all taken.** A private function that lifts the floor exists, so "the
app has no way" was wrong and a fourth option was missing (now D, with Task 2 to try it).
Linux and Windows were already measured, in the document zoom's trial, where the plan said
they were not. The floor had been kept once, on 2026-11-13, which the plan and the item it
is for left out (now *This was known, and kept once*). The table missed the smaller kinds of
text and had `h3` wrong, so the reach runs to 88%, not 72%, and 75% is not clean for all
text. The measurement as first written would have misplaced the floors: text at 9 px is
within 1% of the level just below its floor, and a font's width is not in proportion to its
size (now a control span, in T2). Whether a branch had picked the change across was
answerable, and is answered. Option B's cost was understated against the code. Option A did
not cover Linux. The account of WebKit's overrides missed a second path, which a web view
cannot reach. "Relative units" was not WebKit's rule.

**Pass 2, nine findings, all taken.** Eleven of the fifteen bundled themes set their own
sizes, in `src-tauri/themes/`, which pass 1 had not searched: the table holds for the four
Azure DevOps themes alone, and the fault is smaller in the rest. No `sub` or `small` can
reach a page. One sentence named the wrong kinds of text for 75% and 80%. Option D claimed
as fact what Task 2 has yet to show, and macOS 13 and 14 cannot be tried. Three things that
bear on A against D were missing: the theme, the Macs that get no update, and that D needs
no undoing. T2 could still miss a floor by a level, and its control needed the line's own
font and a measure of the text alone. "The app's own reload" reloads no web view. The
Windows figure is Task 1b's and its half size is inferred. T5 did not say which name.

**Pass 3, five findings, all taken.** The paragraph for the eleven themes left three kinds
of text out at 67% and gave Tufte figures that are not its own. T2's "7% wider" does not
hold above a floor of 76% (6% does), `h3` cannot be tested under 50%, and a level out of
range is turned into 100 silently. A script cannot reach the web view's own menu, and that
it has a Reload on macOS is not known. "Fails safe" covered a function removed, not one
changed. Two sentences gave 64% with no theme named.

**Pass 4, two findings, both wording, both taken.** Two sentences said the private function
can lift the floor, which Task 2 has yet to show; and option B called Tufte a 16 px theme.

**Pass 5: no findings.**
