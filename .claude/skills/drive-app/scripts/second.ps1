param([Parameter(Mandatory)][string]$Path, [string]$Exe = "F:\src\_ pet projects\t4-markdown-viewer\src-tauri\target\debug\t4-markdown-viewer.exe")
# Launch a second instance naming $Path, as a file manager or a terminal would.
# From PowerShell rather than Git Bash, which collapses a leading `\\` and so
# mangles a UNC path. The second instance hands its argv to the running one and
# exits.
& $Exe $Path
"second exit $LASTEXITCODE"
