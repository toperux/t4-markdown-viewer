#!/bin/sh
# The mirror of postinst.sh. Uninstalling deletes the files out of
# /usr/share/mime/packages and /usr/share/applications, but nothing rebuilds the
# caches that were built from them, so the viewer keeps showing up in "Open With"
# and keeps being offered as the handler for .md until something else happens to
# refresh them.
#
# Both are best-effort: a minimal container may not have either tool, and a
# failure here must not fail the package removal.
set -e

if command -v update-mime-database >/dev/null 2>&1; then
  update-mime-database /usr/share/mime || true
fi

if command -v update-desktop-database >/dev/null 2>&1; then
  update-desktop-database /usr/share/applications || true
fi

exit 0
