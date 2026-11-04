; Skip Dat installer (NSIS 3). Build with installer/build.sh.
; Installs for the current Windows account only, so no administrator prompt is needed.

Unicode true
Target amd64-unicode
ManifestDPIAware true
SetCompressor /SOLID lzma
RequestExecutionLevel user

!define APP "Skip Dat"
!ifndef VERSION
  !define VERSION "1.3.0"
!endif
!define EXE "Skip Dat.exe"
!define IDENT "app.skipdat.desktop"
!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\SkipDat"
!define RUN_KEY "Software\Microsoft\Windows\CurrentVersion\Run"
!define WV2_KEY "Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
!ifndef SRC
  !define SRC "stage"
!endif
!ifndef OUTFILE
  !define OUTFILE "SkipDat-${VERSION}-setup.exe"
!endif

Name "${APP}"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\${APP}"
InstallDirRegKey HKCU "${UNINST_KEY}" "InstallLocation"
BrandingText "${APP} ${VERSION}"

VIProductVersion "${VERSION}.0"
VIAddVersionKey "ProductName" "${APP}"
VIAddVersionKey "FileDescription" "${APP} installer"
VIAddVersionKey "FileVersion" "${VERSION}"
VIAddVersionKey "ProductVersion" "${VERSION}"
VIAddVersionKey "LegalCopyright" "${APP}"

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"

; ---------- look ----------
!define MUI_ICON "icon.ico"
!define MUI_UNICON "icon.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_RIGHT
!define MUI_HEADERIMAGE_BITMAP "header.bmp"
!define MUI_HEADERIMAGE_UNBITMAP "header.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP "side.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "side.bmp"
!define MUI_ABORTWARNING
!define MUI_COMPONENTSPAGE_SMALLDESC
ShowInstDetails nevershow
ShowUninstDetails nevershow

; ---------- pages ----------
!define MUI_WELCOMEPAGE_TITLE "Welcome to ${APP}"
!define MUI_WELCOMEPAGE_TEXT "Every music app's queue, one Alt + scroll away.$\r$\n$\r$\nThis installs ${APP} ${VERSION} for your Windows account. No administrator rights are needed.$\r$\n$\r$\nIf ${APP} is running, it will be closed and updated."
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
; while installing: the looping glass animation in place of the details list
!define MUI_PAGE_CUSTOMFUNCTION_SHOW InstFilesShow
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_TITLE "${APP} is ready"
!define MUI_FINISHPAGE_TEXT "${APP} waits in the tray, near the clock. Hold Alt and scroll anywhere to open it.$\r$\n$\r$\nTo see the queues of YouTube Music, YouTube and SoundCloud, add the browser companion. Settings shows how."
!define MUI_FINISHPAGE_TEXT_LARGE
!define MUI_FINISHPAGE_RUN "$INSTDIR\${EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Start ${APP} now"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

; ---------- helpers ----------
!macro CLOSE_RUNNING
  ; the app (and the earlier "Liquid Queue" builds, which would answer the same shortcut)
  nsExec::Exec 'taskkill /IM "${EXE}" /F'
  Pop $0
  nsExec::Exec 'taskkill /IM "liquid-queue.exe" /F'
  Pop $0
  Sleep 500
!macroend

Function InstFilesShow
  ; the progress page's own window, and the space its details list would take
  FindWindow $0 "#32770" "" $HWNDPARENT
  GetDlgItem $1 $0 1016
  System::Call "*(i 0, i 0, i 0, i 0) p .r2"
  System::Call "user32::GetWindowRect(p r1, p r2)"
  System::Call "user32::MapWindowPoints(p 0, p r0, p r2, i 2)"
  System::Call "*$2(i .r3, i .r4, i .r5, i .r6)"
  System::Free $2
  IntOp $5 $5 - $3
  IntOp $6 $6 - $4
  InitPluginsDir
  File "/oname=$PLUGINSDIR\setup-anim.exe" "${SRC}/setup-anim.exe"
  ; a tiny helper draws it on the graphics card and quits when this page closes
  SetDetailsPrint none
  Exec '"$PLUGINSDIR\setup-anim.exe" $0 $3 $4 $5 $6'
  SetDetailsPrint lastused
FunctionEnd

Function EnsureWebView2
  ; the engine that draws the wheel and Settings; part of Windows 11 and up-to-date Windows 10
  ReadRegStr $0 HKLM "SOFTWARE\WOW6432Node\${WV2_KEY}" "pv"
  ${If} $0 == ""
  ${OrIf} $0 == "0.0.0.0"
    ReadRegStr $0 HKLM "SOFTWARE\${WV2_KEY}" "pv"
  ${EndIf}
  ${If} $0 == ""
  ${OrIf} $0 == "0.0.0.0"
    ReadRegStr $0 HKCU "Software\${WV2_KEY}" "pv"
  ${EndIf}
  ${If} $0 == ""
  ${OrIf} $0 == "0.0.0.0"
    DetailPrint "Installing Microsoft Edge WebView2, which ${APP} needs to draw its windows..."
    nsExec::ExecToLog `powershell -NoProfile -ExecutionPolicy Bypass -Command "$$ProgressPreference='SilentlyContinue'; Invoke-WebRequest -UseBasicParsing -Uri 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile '$TEMP\MicrosoftEdgeWebview2Setup.exe'"`
    Pop $1
    ${If} ${FileExists} "$TEMP\MicrosoftEdgeWebview2Setup.exe"
      ExecWait '"$TEMP\MicrosoftEdgeWebview2Setup.exe" /silent /install' $1
      Delete "$TEMP\MicrosoftEdgeWebview2Setup.exe"
    ${Else}
      MessageBox MB_ICONEXCLAMATION|MB_OK "${APP} needs Microsoft Edge WebView2, and it couldn't be downloaded right now.$\r$\n$\r$\nInstall it from https://developer.microsoft.com/microsoft-edge/webview2/ and then start ${APP}." /SD IDOK
    ${EndIf}
  ${EndIf}
FunctionEnd

; ---------- install ----------
Section "${APP}" SecApp
  SectionIn RO
  !insertmacro CLOSE_RUNNING
  !ifdef SLOW
    Sleep 6000   ; test builds only: time to watch the animation
  !endif
  SetOutPath "$INSTDIR"
  File "${SRC}/${EXE}"
  File "${SRC}/WebView2Loader.dll"
  File "/oname=Read me.txt" "${SRC}/readme.txt"
  RMDir /r "$INSTDIR\browser-companion"
  SetOutPath "$INSTDIR\browser-companion"
  File /r "${SRC}/browser-companion/*.*"
  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\Uninstall ${APP}.exe"

  ; Windows Settings > Apps
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "${APP}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "${APP}"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\${EXE},0"
  WriteRegStr HKCU "${UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\Uninstall ${APP}.exe"'
  WriteRegStr HKCU "${UNINST_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall ${APP}.exe" /S'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${UNINST_KEY}" "EstimatedSize" "$0"

  ; "Start with Windows" is set below if chosen; the earlier "Liquid Queue" entry goes away
  DeleteRegValue HKCU "${RUN_KEY}" "${APP}"
  DeleteRegValue HKCU "${RUN_KEY}" "Liquid Queue"

  Call EnsureWebView2
SectionEnd

Section "Start menu shortcut" SecStart
  CreateShortcut "$SMPROGRAMS\${APP}.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${EXE}" 0
SectionEnd

Section /o "Desktop shortcut" SecDesk
  CreateShortcut "$DESKTOP\${APP}.lnk" "$INSTDIR\${EXE}" "" "$INSTDIR\${EXE}" 0
SectionEnd

Section "Start with Windows" SecAuto
  WriteRegStr HKCU "${RUN_KEY}" "${APP}" '"$INSTDIR\${EXE}"'
SectionEnd

!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecApp} "The app, and the browser companion folder for YouTube Music, YouTube and SoundCloud."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecStart} "${APP} in the Start menu."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesk} "${APP} on the desktop."
  !insertmacro MUI_DESCRIPTION_TEXT ${SecAuto} "${APP} starts in the tray when you sign in, ready for the shortcut. You can change this later in Settings."
!insertmacro MUI_FUNCTION_DESCRIPTION_END

; ---------- uninstall ----------
Section "Uninstall"
  !insertmacro CLOSE_RUNNING
  Delete "$INSTDIR\${EXE}"
  Delete "$INSTDIR\WebView2Loader.dll"
  Delete "$INSTDIR\Read me.txt"
  RMDir /r "$INSTDIR\browser-companion"
  Delete "$INSTDIR\Uninstall ${APP}.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\${APP}.lnk"
  Delete "$DESKTOP\${APP}.lnk"
  DeleteRegValue HKCU "${RUN_KEY}" "${APP}"
  DeleteRegKey HKCU "${UNINST_KEY}"
  ; the window engine's cache
  RMDir /r "$LOCALAPPDATA\${IDENT}"
  MessageBox MB_YESNO|MB_ICONQUESTION "Also remove your ${APP} settings (shortcut, hidden apps, Spotify connection)?" /SD IDNO IDNO keep
    RMDir /r "$APPDATA\${IDENT}"
  keep:
SectionEnd
