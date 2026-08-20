; Extra Windows shell registration for file associations.
;
; Tauri's bundler already writes the ProgID and its shell\open\command via
; FileAssociation.nsh. That alone is not enough on Windows 10/11:
;
;   * "Open with" is populated from `.ext\OpenWithProgids` and from
;     `Software\Classes\Applications\<exe>`, neither of which Tauri writes. So
;     the app can be completely absent from the picker.
;   * Settings > Default apps only lists programs that publish a Capabilities
;     key and register it under RegisteredApplications.
;
; What this file cannot do: make the app the default handler. Windows protects
; `FileExts\.md\UserChoice` with a per-user hash and rejects programmatic
; writes. Once a user has opened a .md with anything else, that choice wins
; until they change it themselves. The point of the keys below is to make sure
; the app is actually *offered* when they go to change it.

; Must match bundle.fileAssociations[].name in tauri.conf.json.
!define T4_PROGID "T4MarkdownViewer.Document"
!define T4_CAPKEY "Software\T4MarkdownViewer\Capabilities"

; SHCNE_ASSOCCHANGED — tell Explorer to re-read associations so the change
; shows up without a sign-out.
!define T4_SHCNE_ASSOCCHANGED 0x08000000

!macro T4_REGISTER_EXT EXT
  ; Offer this app in the "Open with" list for the extension.
  WriteRegStr SHELL_CONTEXT "Software\Classes\.${EXT}\OpenWithProgids" "${T4_PROGID}" ""
  ; Declare support on the Applications entry as well; Explorer consults both.
  WriteRegStr SHELL_CONTEXT \
    "Software\Classes\Applications\${MAINBINARYNAME}.exe\SupportedTypes" ".${EXT}" ""
  ; Advertise the association for Settings > Default apps.
  WriteRegStr SHELL_CONTEXT "${T4_CAPKEY}\FileAssociations" ".${EXT}" "${T4_PROGID}"
!macroend

!macro T4_UNREGISTER_EXT EXT
  DeleteRegValue SHELL_CONTEXT "Software\Classes\.${EXT}\OpenWithProgids" "${T4_PROGID}"
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; --- Applications entry: what "Open with > Choose another app" reads. ---
  WriteRegStr SHELL_CONTEXT \
    "Software\Classes\Applications\${MAINBINARYNAME}.exe" "FriendlyAppName" "${PRODUCTNAME}"
  WriteRegStr SHELL_CONTEXT \
    "Software\Classes\Applications\${MAINBINARYNAME}.exe\shell\open\command" "" \
    '"$INSTDIR\${MAINBINARYNAME}.exe" "%1"'
  WriteRegStr SHELL_CONTEXT \
    "Software\Classes\Applications\${MAINBINARYNAME}.exe\DefaultIcon" "" \
    "$INSTDIR\${MAINBINARYNAME}.exe,0"

  ; A friendly type name so Explorer's Type column reads well.
  WriteRegStr SHELL_CONTEXT "Software\Classes\${T4_PROGID}" "FriendlyTypeName" "Markdown Document"

  ; --- Per-extension registration. Keep in sync with tauri.conf.json. ---
  !insertmacro T4_REGISTER_EXT "md"
  !insertmacro T4_REGISTER_EXT "markdown"
  !insertmacro T4_REGISTER_EXT "mdown"
  !insertmacro T4_REGISTER_EXT "mkd"
  !insertmacro T4_REGISTER_EXT "mdtext"

  ; --- Capabilities: required to appear in Settings > Default apps. ---
  WriteRegStr SHELL_CONTEXT "${T4_CAPKEY}" "ApplicationName" "${PRODUCTNAME}"
  WriteRegStr SHELL_CONTEXT "${T4_CAPKEY}" "ApplicationDescription" \
    "A fast, themeable Markdown viewer."
  WriteRegStr SHELL_CONTEXT "Software\RegisteredApplications" "${PRODUCTNAME}" "${T4_CAPKEY}"

  System::Call 'shell32::SHChangeNotify(i ${T4_SHCNE_ASSOCCHANGED}, i 0, i 0, i 0)'
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro T4_UNREGISTER_EXT "md"
  !insertmacro T4_UNREGISTER_EXT "markdown"
  !insertmacro T4_UNREGISTER_EXT "mdown"
  !insertmacro T4_UNREGISTER_EXT "mkd"
  !insertmacro T4_UNREGISTER_EXT "mdtext"

  DeleteRegKey SHELL_CONTEXT "Software\Classes\Applications\${MAINBINARYNAME}.exe"
  DeleteRegValue SHELL_CONTEXT "Software\RegisteredApplications" "${PRODUCTNAME}"
  DeleteRegKey SHELL_CONTEXT "Software\T4MarkdownViewer"

  System::Call 'shell32::SHChangeNotify(i ${T4_SHCNE_ASSOCCHANGED}, i 0, i 0, i 0)'
!macroend
