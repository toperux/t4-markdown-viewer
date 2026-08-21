#!/bin/sh
# Rebuild the caches that make the file association visible. Dropping files into
# /usr/share/mime/packages and /usr/share/applications has no effect until these
# run, so without this the viewer installs but never appears in "Open With".
#
# Both are best-effort: a minimal container may not have either tool, and a
# failure here must not fail the package install.
set -e

if command -v update-mime-database >/dev/null 2>&1; then
  update-mime-database /usr/share/mime || true
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications || true
fi

exit 0
