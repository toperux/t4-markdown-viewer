param([string]$Out, [string]$Exe = "F:\src\_ pet projects\t4-markdown-viewer\src-tauri\target\debug\t4-markdown-viewer.exe")
# Capture the debug app's main window with PrintWindow (full content), and print its rect.
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public class W {
  [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Rr, B; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint f);
}
"@
$p = Get-Process t4-markdown-viewer | Where-Object { $_.Path -eq $Exe } | Select-Object -First 1
$h = $p.MainWindowHandle
$r = New-Object W+R; [W]::GetWindowRect($h, [ref]$r) | Out-Null
"pid=$($p.Id) rect=$($r.L),$($r.T),$($r.Rr),$($r.B) title=$($p.MainWindowTitle)"
$bmp = New-Object System.Drawing.Bitmap ($r.Rr - $r.L), ($r.B - $r.T)
$g = [System.Drawing.Graphics]::FromImage($bmp); $dc = $g.GetHdc()
[W]::PrintWindow($h, $dc, 2) | Out-Null
$g.ReleaseHdc($dc); $bmp.Save($Out); $g.Dispose(); $bmp.Dispose()
