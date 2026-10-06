param([int]$ProcId, [long]$Hwnd = 0, [string]$Keys)
# Bring one of the app's windows to the front and type $Keys through the OS
# (SendKeys syntax: ^r is Ctrl+R), so WebView2's browser accelerators see a
# real key press — CDP's Input.dispatchKeyEvent goes straight to the renderer.
# -Hwnd picks the window; without it, the process's main window. Refused
# (exit 2) unless that window is the foreground window just before typing:
# Windows can refuse the switch, and the keys would then land in whatever app
# has focus, such as the user's editor.
Add-Type @"
using System; using System.Runtime.InteropServices;
public class Fg {
  [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
}
"@
Add-Type -AssemblyName System.Windows.Forms
$h = if ($Hwnd) { [IntPtr]$Hwnd } else { (Get-Process -Id $ProcId).MainWindowHandle }
# A zero-key event first: Windows lets the last process with input take the
# foreground.
[Fg]::keybd_event(0, 0, 0, [UIntPtr]::Zero)
[void][Fg]::SetForegroundWindow($h)
Start-Sleep -Milliseconds 300
if ([Fg]::GetForegroundWindow() -ne $h) { "REFUSED: foreground is " + [Fg]::GetForegroundWindow() + ", not " + $h; exit 2 }
[System.Windows.Forms.SendKeys]::SendWait($Keys)
"sent $Keys"
