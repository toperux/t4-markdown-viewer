param([int]$ProcId, [string]$Keys)
# Bring the process's main window to the front and type $Keys through the OS
# (SendKeys syntax: ^r is Ctrl+R), so WebView2's browser accelerators see a
# real key press — CDP's Input.dispatchKeyEvent goes straight to the renderer.
$sh = New-Object -ComObject WScript.Shell
"activated=" + $sh.AppActivate($ProcId)
Start-Sleep -Milliseconds 400
$sh.SendKeys($Keys)
"sent $Keys"
