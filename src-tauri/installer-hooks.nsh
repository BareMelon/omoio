; Uninstall behaviour, which the generated script gets wrong for this app in
; two ways.
;
; First, it deletes the executable and then calls RMDir without /r, which only
; removes an empty folder. Omoio keeps the RPCS3 build it downloaded inside its
; own install folder, so that call fails quietly and leaves 70 MB of emulator
; behind for good.
;
; Second, the "delete application data" checkbox removes $APPDATA\<bundle id>,
; and this app stores nothing there. Its library, save backups, logs and cover
; art live in $APPDATA\Omoio, so ticking the box removed nothing at all while
; telling the user otherwise.
;
; The emulator is ours and always goes. Their own things are only removed if
; they asked, using the checkbox that already exists rather than a second
; question. Their games are never touched: those live wherever they put them,
; and Omoio only ever referenced them.

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    SetShellVarContext current

    ; The emulator we downloaded and managed.
    RMDir /r "$INSTDIR\rpcs3"
    RMDir /r "$INSTDIR"

    ${If} $DeleteAppDataCheckboxState = 1
      RMDir /r "$APPDATA\Omoio"
    ${EndIf}
  ${EndIf}
!macroend
