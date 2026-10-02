; Uninstall behaviour, which the generated script gets wrong for this app in
; two ways.
;
; First, it deletes the executable and then calls RMDir without /r, which only
; removes an empty folder. Omoio keeps the emulators it downloaded inside its
; own install folder, so that call fails quietly and leaves them behind.
;
; Second, the "delete application data" checkbox removes $APPDATA\<bundle id>,
; and this app stores nothing there. Its library, save backups, logs and cover
; art live in $APPDATA\Omoio, so ticking the box removed nothing at all while
; telling the user otherwise.
;
; The emulators hold the games' own saves (RPCS3's dev_hdd0, Cemu's
; portable\mlc01) and the user's Wii U keys, so they go only with the user's
; other things, when the checkbox that already exists is ticked. Left unticked,
; they stay for a reinstall to find. Their games are never touched: those live
; wherever they put them, and Omoio only ever referenced them.

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    SetShellVarContext current

    ${If} $DeleteAppDataCheckboxState = 1
      RMDir /r "$INSTDIR"
      RMDir /r "$APPDATA\Omoio"
    ${EndIf}
  ${EndIf}
!macroend
