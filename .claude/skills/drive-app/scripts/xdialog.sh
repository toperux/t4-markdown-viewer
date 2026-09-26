#!/usr/bin/env bash
# Answers the app's GTK Open / Select Folder dialog under Xvfb, the Linux
# counterpart of dialog.ps1:
#   xdialog.sh <pid> <path> | xdialog.sh <pid> --cancel | xdialog.sh <pid> --dump
# Keys go through XTEST to the focused window: `xdotool key --window` sends
# synthetic events, which GTK ignores. No window manager, so focus, not activate.
set -euo pipefail
# Not $DISPLAY: the desktop session always sets that, to its own display.
export DISPLAY="${XDISPLAY:-:99}"
pid=$1 what=$2
if [[ $what == --dump ]]; then
  wins=$(xdotool search --onlyvisible --pid "$pid" 2>/dev/null || true)
  for w in $wins; do echo "$w $(xdotool getwindowname "$w")"; done
  exit
fi
# By title: GTK also maps untitled helper windows, so "not the main window"
# picks the wrong one. The app sets no title, so these are GTK's own. --all:
# xdotool ORs its conditions otherwise.
find_dialog() {
  xdotool search --all --onlyvisible --pid "$pid" --name '^(Open File|Select Folder)$' 2>/dev/null | head -1 | grep .
}
dlg=$(find_dialog || true)
[[ -n $dlg ]] || { echo "no dialog open for pid $pid"; exit 1; }
xdotool windowfocus --sync "$dlg"
if [[ $what == --cancel ]]; then
  xdotool key Escape
else
  # Ctrl+L opens the location box; typing a path then Return picks it.
  xdotool key ctrl+l
  xdotool type --delay 5 "$what"
  xdotool key Return
fi
# GTK takes a moment to unmap it.
for _ in 1 2 3 4 5 6; do
  sleep 0.5
  find_dialog >/dev/null || { echo "dialog closed"; exit; }
done
echo "dialog still open"
