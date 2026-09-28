#!/usr/bin/env bash
# Screen points of the centre of every element matching a CSS selector, for
# macmouse.js: macpt.sh "<css selector>" → ["x,y", …]
# The window rect is the outer frame in physical px; the content starts
# below the title bar, i.e. the frame's height minus innerHeight.
# window_origin gives the frame's top too, not the content's (measured).
set -euo pipefail
W="$(dirname "$0")/wd.mjs"
export WD_PORT="${WD_PORT:-4445}"
# JSON-encoded, so quotes and backslashes in the selector survive as typed.
sel=$(node -p 'JSON.stringify(process.argv[1])' "$1")
r=$(node "$W" raw GET /window/rect) || { echo "$r"; exit 1; }
node "$W" eval "(() => { const r = $r, s = devicePixelRatio, top = r.y / s + r.height / s - innerHeight;
  return [...document.querySelectorAll($sel)].map((e) => { const b = e.getBoundingClientRect();
    return Math.round(r.x / s + b.x + b.width / 2) + ',' + Math.round(top + b.y + b.height / 2); }); })()"
