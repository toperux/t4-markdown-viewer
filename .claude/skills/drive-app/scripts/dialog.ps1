# Answer the app's native open dialog with plain Win32 messages.
# UI Automation does not expose the dialog's file-name box or Open/Cancel buttons.
# dialog.ps1 -ProcId <pid> -Path <file or folder>   |   -Cancel   |   -Dump
param([int]$ProcId, [string]$Path, [switch]$Cancel, [switch]$Dump)
Add-Type @"
using System; using System.Text; using System.Collections.Generic; using System.Runtime.InteropServices;
public static class W {
  public delegate bool EnumProc(IntPtr h, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
  [DllImport("user32.dll")] static extern bool EnumChildWindows(IntPtr p, EnumProc f, IntPtr l);
  [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetClassName(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr h, StringBuilder s, int n);
  [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
  [DllImport("user32.dll")] public static extern int GetDlgCtrlID(IntPtr h);
  [DllImport("user32.dll", CharSet = CharSet.Unicode)] public static extern IntPtr SendMessage(IntPtr h, uint m, IntPtr w, string l);
  [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr h, uint m, IntPtr w, IntPtr l);
  public static string Cls(IntPtr h) { var s = new StringBuilder(256); GetClassName(h, s, 256); return s.ToString(); }
  public static string Txt(IntPtr h) { var s = new StringBuilder(256); GetWindowText(h, s, 256); return s.ToString(); }
  public static IntPtr FindDialog(uint pid) {
    IntPtr found = IntPtr.Zero;
    EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p);
      if (p == pid && IsWindowVisible(h) && Cls(h) == "#32770") { found = h; return false; } return true; }, IntPtr.Zero);
    return found;
  }
  public static List<IntPtr> Children(IntPtr parent) {
    var list = new List<IntPtr>(); EnumChildWindows(parent, (h, l) => { list.Add(h); return true; }, IntPtr.Zero); return list;
  }
}
"@
$WM_SETTEXT = 0x000C; $WM_COMMAND = 0x0111

$dlg = [IntPtr]::Zero
for ($i = 0; $i -lt 40 -and $dlg -eq [IntPtr]::Zero; $i++) { $dlg = [W]::FindDialog($ProcId); if ($dlg -eq [IntPtr]::Zero) { Start-Sleep -Milliseconds 250 } }
if ($dlg -eq [IntPtr]::Zero) { 'no dialog'; exit 1 }
"dialog: '$([W]::Txt($dlg))'"
$kids = [W]::Children($dlg)
function Info($h) { "{0} id={1} text='{2}' visible={3}" -f [W]::Cls($h), [W]::GetDlgCtrlID($h), [W]::Txt($h), [W]::IsWindowVisible($h) }

if ($Dump) { foreach ($h in $kids) { if ([W]::Cls($h) -in 'Edit', 'Button', 'ComboBoxEx32', 'ComboBox') { Info $h } }; exit }

# The command a button sends its dialog when clicked: WM_COMMAND, id in the low word, BN_CLICKED (0) high.
function Click($id) {
  $btn = $kids | Where-Object { [W]::Cls($_) -eq 'Button' -and [W]::GetDlgCtrlID($_) -eq $id } | Select-Object -First 1
  "button: $(Info $btn)"
  [void][W]::PostMessage($dlg, $WM_COMMAND, [IntPtr]$id, $btn)
}

if ($Cancel) { Click 2 } else {
  # The file dialog's name box: an Edit inside the ComboBoxEx32 with id 1148.
  $combo = $kids | Where-Object { [W]::Cls($_) -eq 'ComboBoxEx32' -and [W]::GetDlgCtrlID($_) -eq 1148 } | Select-Object -First 1
  $edit = if ($combo) { [W]::Children($combo) | Where-Object { [W]::Cls($_) -eq 'Edit' } | Select-Object -First 1 }
  # The folder picker carries a bare Edit (id 1152) instead.
  if (-not $edit) { $edit = $kids | Where-Object { [W]::Cls($_) -eq 'Edit' -and [W]::IsWindowVisible($_) } | Select-Object -First 1 }
  "edit: $(Info $edit)"
  [void][W]::SendMessage($edit, $WM_SETTEXT, [IntPtr]::Zero, $Path)
  Click 1
}
Start-Sleep -Milliseconds 800
if ([W]::FindDialog($ProcId) -ne [IntPtr]::Zero) { 'dialog still open' } else { 'dialog closed' }
