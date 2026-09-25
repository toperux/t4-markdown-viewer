param([long]$Hwnd)
# Resize the window as a user would: put it at 100,100 800x600, bring it to the
# front, then drag its bottom-right corner up-left to its top-left. A user drag
# runs the modal size loop, which is what honours the minimum track size;
# MoveWindow does not.
Add-Type -Namespace W -Name D -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool MoveWindow(System.IntPtr h, int x, int y, int w, int hh, bool r);
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(System.IntPtr h);
[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
[DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, System.IntPtr e);
[StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
[DllImport("user32.dll")] public static extern bool GetWindowRect(System.IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool GetClientRect(System.IntPtr h, out RECT r);
'@
$h = [IntPtr]$Hwnd
[void][W.D]::MoveWindow($h, 100, 100, 800, 600, $true)
$sh = New-Object -ComObject WScript.Shell
[void]$sh.SendKeys('%')
[void][W.D]::SetForegroundWindow($h)
Start-Sleep -Milliseconds 400
$r = New-Object W.D+RECT; [void][W.D]::GetWindowRect($h, [ref]$r)
# The corner's resize zone sits in the invisible border just outside the frame.
$x = $r.R - 4; $y = $r.B - 4
[void][W.D]::SetCursorPos($x, $y); Start-Sleep -Milliseconds 200
[W.D]::mouse_event(2, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 200
for ($i = 1; $i -le 20; $i++) {
  [void][W.D]::SetCursorPos($x - ($x - $r.L - 20) * $i / 20, $y - ($y - $r.T - 20) * $i / 20)
  Start-Sleep -Milliseconds 30
}
[W.D]::mouse_event(4, 0, 0, 0, [IntPtr]::Zero); Start-Sleep -Milliseconds 400
$c = New-Object W.D+RECT; [void][W.D]::GetClientRect($h, [ref]$c)
"client after drag=$($c.R)x$($c.B)"
