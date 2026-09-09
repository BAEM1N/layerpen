; The custom template keeps the old registry identity; all visible branding is Pointory.
; Use Tauri's normal running-app prompt before replacing a previous binary.
!macro NSIS_HOOK_PREINSTALL
  ReadRegStr $OldMainBinaryName SHCTX "${UNINSTKEY}" "MainBinaryName"
  ${If} $OldMainBinaryName != ""
  ${AndIf} $OldMainBinaryName != "${MAINBINARYNAME}.exe"
    !insertmacro CheckIfAppIsRunning "$OldMainBinaryName" "${PRODUCTNAME}"
  ${EndIf}
!macroend

; Touch only shortcuts that actually target this install's previous executable.
!macro PointoryMigrateShortcut oldPath newPath
  !insertmacro IsShortcutTarget "${oldPath}" "$INSTDIR\$OldMainBinaryName"
  Pop $0
  ${If} $0 = 1
    !insertmacro UnpinShortcut "${oldPath}"
    ${If} ${FileExists} "${newPath}"
      Delete "${oldPath}"
    ${Else}
      Rename "${oldPath}" "${newPath}"
      !insertmacro SetShortcutTarget "${newPath}" "$INSTDIR\${MAINBINARYNAME}.exe"
      !insertmacro SetLnkAppUserModelId "${newPath}"
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ${If} $OldMainBinaryName != ""
  ${AndIf} $OldMainBinaryName != "${MAINBINARYNAME}.exe"
    !insertmacro PointoryMigrateShortcut "$SMPROGRAMS\$AppStartMenuFolder\OnPen.lnk" "$SMPROGRAMS\$AppStartMenuFolder\${PRODUCTNAME}.lnk"
    !insertmacro PointoryMigrateShortcut "$SMPROGRAMS\OnPen.lnk" "$SMPROGRAMS\${PRODUCTNAME}.lnk"
    !insertmacro PointoryMigrateShortcut "$DESKTOP\OnPen.lnk" "$DESKTOP\${PRODUCTNAME}.lnk"
  ${EndIf}
!macroend
