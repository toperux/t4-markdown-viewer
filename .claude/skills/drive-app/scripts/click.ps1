param([int]$ProcId, [int]$X, [int]$Y)
# Bring the process's window to the front, then a real left click at screen X,Y.
Add-Type @"
using System; using System.Runtime.InteropServices;
public class M {
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, uint d, UIntPtr e);
}
"@
(New-Object -ComObject WScript.Shell).AppActivate($ProcId) | Out-Null
Start-Sleep -Milliseconds 400
[M]::SetCursorPos($X, $Y) | Out-Null
Start-Sleep -Milliseconds 100
[M]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero); [M]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
