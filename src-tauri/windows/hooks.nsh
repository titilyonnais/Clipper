; Uninstall: give Win+V back to Windows if Clipper took it over, by removing
; only the letter V from Explorer's disabled hotkeys (other letters are kept).
; Takes effect at the next sign-in or Explorer restart.
!macro NSIS_HOOK_POSTUNINSTALL
  ReadRegDWORD $0 HKCU "Software\Clipper" "WinVTakenOver"
  ${If} $0 == 1
    ReadRegStr $1 HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced" "DisabledHotkeys"
    StrCpy $2 ""
    StrLen $3 $1
    StrCpy $4 0
    clipper_winv_loop:
      IntCmp $4 $3 clipper_winv_done
      StrCpy $5 $1 1 $4
      StrCmp $5 "V" clipper_winv_next
      StrCpy $2 "$2$5"
      clipper_winv_next:
      IntOp $4 $4 + 1
      Goto clipper_winv_loop
    clipper_winv_done:
    ${If} $2 == ""
      DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced" "DisabledHotkeys"
    ${Else}
      WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced" "DisabledHotkeys" $2
    ${EndIf}
  ${EndIf}
  DeleteRegKey HKCU "Software\Clipper"
!macroend
