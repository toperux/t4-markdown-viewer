#!/usr/bin/env bash
# Answers the app's Open / Select Folder panel on macOS, the counterpart of
# xdialog.sh and dialog.ps1:
#   macdialog.sh <pid> <path> | macdialog.sh <pid> --cancel | macdialog.sh <pid> --dump
# The panel is a sheet on the window it was opened from, which need not be the
# front one. Keys go through System Events to the key window, so that window is
# raised and the app brought to the front first. Exits 1 when nothing was done.
# Needs Accessibility and Automation (System Events) for the terminal.
set -euo pipefail
pid=$1 what=$2
osascript - "$pid" "$what" <<'EOF'
on run argv
  set pid to (item 1 of argv) as integer
  set what to item 2 of argv
  tell application "System Events"
    set p to first process whose unix id is pid
    if what is "--dump" then
      -- The panel runs out of process: its buttons aren't visible here.
      set out to ""
      repeat with w in windows of p
        set out to out & "window: " & name of w
        if exists sheet 1 of w then set out to out & " (sheet open)"
        set out to out & linefeed
      end repeat
      return out
    end if
    set dlg to missing value
    repeat with w in windows of p
      if exists sheet 1 of w then
        set dlg to contents of w
        exit repeat
      end if
    end repeat
    if dlg is missing value then error "no dialog open for pid " & pid number 1
    set frontmost of p to true
    perform action "AXRaise" of dlg
    delay 0.3
    if not frontmost of p then error "app is not in front; nothing typed" number 1
    if what is "--cancel" then
      key code 53
    else
      -- Go to Folder, then the path; Return goes there, a second Return opens.
      -- keystroke types through the keyboard layout: ASCII paths only.
      keystroke "g" using {command down, shift down}
      delay 0.5
      keystroke what
      delay 0.5
      key code 36
      delay 0.8
      key code 36
    end if
    -- Any window, not dlg: raising it may renumber a specifier held by index.
    repeat 6 times
      delay 0.5
      set still to false
      repeat with w in windows of p
        if exists sheet 1 of w then set still to true
      end repeat
      if not still then return "dialog closed"
    end repeat
    return "dialog still open"
  end tell
end run
EOF
