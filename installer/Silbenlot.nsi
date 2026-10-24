Unicode true
!if "${ARCH}" == "x64"
  Target amd64-unicode
!else
  Target x86-unicode
!endif
!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "x64.nsh"
!include "WinVer.nsh"
!include "WordFunc.nsh"
Name "Silbenlot"
OutFile "${OUTPUT}"
InstallDir "$LOCALAPPDATA\Programs\Silbenlot"
RequestExecutionLevel user
SetCompressor /FINAL zlib
CRCCheck force
ManifestDPIAware true
BrandingText "Silbenlot · Lokales Diktieren"
VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=1031 "ProductName" "Silbenlot Setup"
VIAddVersionKey /LANG=1031 "FileDescription" "Silbenlot Installation (${ARCH})"
VIAddVersionKey /LANG=1031 "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=1031 "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=1031 "LegalCopyright" "GPL-3.0; beiliegende Drittanbieter-Lizenzen"
!define MUI_ICON "${ICON}"
!define MUI_UNICON "${ICON}"
!define MUI_ABORTWARNING
!define MUI_WELCOMEPAGE_TITLE "Silbenlot installieren"
!define MUI_WELCOMEPAGE_TEXT "Dieses Setup installiert Silbenlot, Parakeet v3 und alle benötigten Laufzeiten für dein Benutzerkonto.$\r$\n$\r$\nKeine zusätzlichen Downloads erforderlich. Eine laufende Silbenlot-Version wird regulär beendet. Deine Diktate und Einstellungen bleiben erhalten."
!define MUI_COMPONENTSPAGE_TEXT_TOP "Die App und der Startmenü-Eintrag werden installiert. Ein Desktop-Symbol ist optional."
!define MUI_FINISHPAGE_TITLE "Installation abgeschlossen"
!define MUI_FINISHPAGE_TEXT "Silbenlot ist installiert. Du findest die App im Windows-Startmenü.$\r$\n$\r$\nBei einem Startproblem liegt die Diagnose unter %LOCALAPPDATA%\Silbenlot\ui-start.log."
!define MUI_FINISHPAGE_RUN "$INSTDIR\Silbenlot.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Silbenlot jetzt öffnen"
!insertmacro MUI_PAGE_WELCOME
!define MUI_LICENSEPAGE_CHECKBOX
!define MUI_LICENSEPAGE_TEXT_TOP "Bitte die Bedingungen der separat mitgelieferten Microsoft-Runtime lesen. Die GPL-Rechte am Anwendungscode bleiben bestehen."
!insertmacro MUI_PAGE_LICENSE "${STAGE}/licenses/Runtime-Distribution-Notice.txt"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH
!insertmacro MUI_LANGUAGE "German"

Function .onInit
  SetShellVarContext current
  SetRegView 64
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_OK|MB_ICONSTOP "Silbenlot benötigt Windows 10 oder Windows 11." /SD IDOK
    SetErrorLevel 2
    Abort
  ${EndIf}
!if "${ARCH}" == "ARM64"
  ${IfNot} ${IsNativeARM64}
    MessageBox MB_OK|MB_ICONSTOP "Dieses Setup ist für Windows auf ARM. Für Intel/AMD bitte Silbenlot-Setup-x64.exe verwenden." /SD IDOK
    SetErrorLevel 2
    Abort
  ${EndIf}
!else
  ${IfNot} ${IsNativeAMD64}
    MessageBox MB_OK|MB_ICONSTOP "Dieses Setup ist für Intel/AMD (64 Bit). Für Windows auf ARM bitte Silbenlot-Setup-ARM64.exe verwenden." /SD IDOK
    SetErrorLevel 2
    Abort
  ${EndIf}
!endif
  ReadRegStr $0 HKCU "Software\Silbenlot" "Version"
  ${If} $0 != ""
    ${VersionCompare} "$0" "${VERSION}" $1
    ${If} $1 == 1
      MessageBox MB_OK|MB_ICONSTOP "Eine neuere Silbenlot-Version ist bereits installiert." /SD IDOK
      SetErrorLevel 2
      Abort
    ${EndIf}
  ${EndIf}
  ReadRegStr $0 HKCU "Software\Silbenlot" "InstallDir"
  ${If} $0 != ""
    StrCpy $INSTDIR $0
  ${EndIf}
FunctionEnd

Function StopRunning
  InitPluginsDir
  SetOutPath "$PLUGINSDIR"
  File /oname=Silbenlot-control.exe "${CORE}"
 retry:
  ClearErrors
  ExecWait '"$PLUGINSDIR\Silbenlot-control.exe" --quit-existing' $0
  IfErrors failed
  StrCmp $0 0 done
 failed:
  IfSilent abort
  MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Silbenlot läuft noch oder schließt ein Diktat ab. Bitte kurz warten oder die alte App über ihr Tray-Menü beenden." IDRETRY retry
 abort:
  SetErrorLevel 2
  Abort
 done:
FunctionEnd

Section "Silbenlot und Startmenü (erforderlich)" MainSection
  SectionIn RO
  Call StopRunning
  SetDetailsPrint textonly
  DetailPrint "Programmdateien und lokales Sprachmodell installieren …"
  SetDetailsPrint listonly
  !include "${META}/payload.nsh"
  SetDetailsPrint textonly
  DetailPrint "Lokale Oberflächen-Laufzeit vorbereiten …"
  SetDetailsPrint listonly
  nsExec::ExecToLog '"$SYSDIR\icacls.exe" "$INSTDIR\webview2" /grant *S-1-15-2-2:(OI)(CI)(RX) *S-1-15-2-1:(OI)(CI)(RX) /T /Q'
  Pop $0
  ${If} $0 != 0
    MessageBox MB_OK|MB_ICONSTOP "Die Leserechte für die lokale Oberfläche konnten nicht gesetzt werden. Bitte das Setup erneut ausführen." /SD IDOK
    SetErrorLevel 2
    Abort
  ${EndIf}
  FileOpen $0 "$INSTDIR\webview2\.diktator-appcontainer-rx" w
  FileWrite $0 "runtime-only RX"
  FileClose $0
  SetDetailsPrint textonly
  DetailPrint "Installierte Dateien prüfen …"
  SetDetailsPrint listonly
  ClearErrors
  ExecWait '"$INSTDIR\Silbenlot.exe" --verify-install' $0
  IfErrors verifyFailed
  StrCmp $0 0 verified
 verifyFailed:
  MessageBox MB_OK|MB_ICONSTOP "Die Programmdateien konnten nicht vollständig geprüft werden. Bitte das Setup erneut ausführen." /SD IDOK
  SetErrorLevel 2
  Abort
 verified:
  SetOutPath "$INSTDIR"
  WriteUninstaller "$INSTDIR\Uninstall.exe"
  CreateDirectory "$SMPROGRAMS\Silbenlot"
  CreateShortCut "$SMPROGRAMS\Silbenlot\Silbenlot.lnk" "$INSTDIR\Silbenlot.exe" "" "$INSTDIR\Silbenlot.exe" 0
  CreateShortCut "$SMPROGRAMS\Silbenlot\Silbenlot deinstallieren.lnk" "$INSTDIR\Uninstall.exe"
  WriteRegStr HKCU "Software\Silbenlot" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Silbenlot" "Version" "${VERSION}"
  WriteRegStr HKCU "Software\Silbenlot" "Architecture" "${ARCH}"
  ; Migrate only an exact registered predecessor command with our install marker.
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Diktator"
  ReadRegStr $1 HKCU "Software\Diktator" "InstallDir"
  ${If} $1 != ""
    IfFileExists "$1\.diktator-installed.json" 0 legacyRunDone
    StrCmp $0 '$\"$1\Diktator.exe$\" --autostart' 0 legacyRunDone
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Silbenlot" '$\"$INSTDIR\Silbenlot.exe$\" --autostart'
    DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Diktator"
  ${EndIf}
 legacyRunDone:
  ; Preserve an already enabled autostart while moving from a portable path.
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Silbenlot"
  ${If} $0 != ""
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Silbenlot" '$\"$INSTDIR\Silbenlot.exe$\" --autostart'
  ${EndIf}
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Silbenlot" "DisplayName" "Silbenlot"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Silbenlot" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Silbenlot" "Publisher" "Silbenlot"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Silbenlot" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Silbenlot" "DisplayIcon" '$\"$INSTDIR\Silbenlot.exe$\",0'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Silbenlot" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Silbenlot" "QuietUninstallString" '$\"$INSTDIR\Uninstall.exe$\" /S'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Silbenlot" "EstimatedSize" ${SIZE_KB}
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Silbenlot" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Silbenlot" "NoRepair" 1
SectionEnd

Section /o "Desktop-Verknüpfung" DesktopSection
  CreateShortCut "$DESKTOP\Silbenlot.lnk" "$INSTDIR\Silbenlot.exe" "" "$INSTDIR\Silbenlot.exe" 0
SectionEnd

Function un.onInit
  SetShellVarContext current
  SetRegView 64
  ReadRegStr $0 HKCU "Software\Silbenlot" "InstallDir"
  ${If} $0 != $INSTDIR
    MessageBox MB_OK|MB_ICONSTOP "Dieses Deinstallationsprogramm gehört nicht zur aktuell registrierten Installation." /SD IDOK
    SetErrorLevel 2
    Abort
  ${EndIf}
FunctionEnd

Section "Uninstall"
 retry:
  IfFileExists "$INSTDIR\Silbenlot.exe" 0 closed
  ClearErrors
  ExecWait '"$INSTDIR\Silbenlot.exe" --quit-existing' $0
  IfErrors failed
  StrCmp $0 0 closed
 failed:
  IfSilent abort
  MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "Bitte Silbenlot vollständig schließen und erneut versuchen." IDRETRY retry
 abort:
  SetErrorLevel 2
  Abort
 closed:
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Silbenlot"
  StrCmp $0 '$\"$INSTDIR\Silbenlot.exe$\" --autostart' 0 +2
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Silbenlot"
  Delete "$DESKTOP\Silbenlot.lnk"
  Delete "$SMPROGRAMS\Silbenlot\Silbenlot.lnk"
  Delete "$SMPROGRAMS\Silbenlot\Silbenlot deinstallieren.lnk"
  RMDir "$SMPROGRAMS\Silbenlot"
  !include "${META}/remove.nsh"
  Delete "$INSTDIR\.verified-cache.json"
  Delete "$INSTDIR\webview2\.diktator-appcontainer-rx"
  RMDir "$INSTDIR\webview2"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\Silbenlot"
  DeleteRegKey HKCU "Software\Silbenlot"
  ; Deliberately retain %LOCALAPPDATA%\Silbenlot: history/settings belong to the user.
SectionEnd
