; EasySSH Lite NSIS Installer Script
; Minimal SSH configuration vault
;
; Build command:
;   makensis /DPRODUCT_VERSION=0.3.0 easyssh-lite.nsi

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"
!include "WinVer.nsh"

;--------------------------------
; General Configuration
;--------------------------------
!ifndef PRODUCT_VERSION
  !define PRODUCT_VERSION "0.3.0"
!endif

Name "EasySSH Lite"
OutFile "EasySSH-Lite-${PRODUCT_VERSION}-x64.exe"
InstallDir "$LOCALAPPDATA\Programs\EasySSH Lite"
InstallDirRegKey HKCU "Software\AnixOps\EasySSHLite" "InstallDir"
RequestExecutionLevel user

;--------------------------------
; Version Information
;--------------------------------
VIProductVersion "${PRODUCT_VERSION}.0"
VIAddVersionKey "ProductName" "EasySSH Lite"
VIAddVersionKey "CompanyName" "AnixOps"
VIAddVersionKey "LegalCopyright" "Copyright (c) 2026 AnixOps"
VIAddVersionKey "FileDescription" "EasySSH Lite - SSH Configuration Vault"
VIAddVersionKey "FileVersion" "${PRODUCT_VERSION}"
VIAddVersionKey "ProductVersion" "${PRODUCT_VERSION}"
VIAddVersionKey "InternalName" "EasySSHLite"
VIAddVersionKey "OriginalFilename" "EasySSH-Lite.exe"

;--------------------------------
; MUI Configuration
;--------------------------------
!define MUI_ICON "..\..\..\..\crates\easyssh-platforms\windows\easyssh-winui\assets\icon.ico"
!define MUI_UNICON "..\..\..\..\crates\easyssh-platforms\windows\easyssh-winui\assets\icon.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "images\header.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP "images\welcome.bmp"

;--------------------------------
; Pages
;--------------------------------
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\..\..\..\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_NOAUTOCLOSE
!define MUI_FINISHPAGE_RUN "$INSTDIR\EasySSH Lite.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch EasySSH Lite"
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_UNPAGE_WELCOME
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

;--------------------------------
; Languages
;--------------------------------
!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "SimplifiedChinese"
!insertmacro MUI_LANGUAGE "TraditionalChinese"

;--------------------------------
; Sections
;--------------------------------
Section "EasySSH Lite (Required)" SecApp
  SectionIn RO

  SetOutPath "$INSTDIR"

  ; Main executable
  File "..\..\..\..\target\release-lite\easyssh-lite.exe"
  File "..\..\..\..\crates\easyssh-platforms\windows\easyssh-winui\assets\icon.ico"
  File "..\..\..\..\LICENSE"

  ; Create README
  FileOpen $0 "$INSTDIR\README.txt" w
  FileWrite $0 "EasySSH Lite v${PRODUCT_VERSION}$
"
  FileWrite $0 "===================$
"
  FileWrite $0 "$
"
  FileWrite $0 "Quick Start:$
"
  FileWrite $0 "1. Run EasySSH Lite.exe$
"
  FileWrite $0 "2. Add your SSH servers via the UI$
"
  FileWrite $0 "3. Connect using password or key authentication$
"
  FileWrite $0 "4. Launches your native terminal (Windows Terminal recommended)$
"
  FileWrite $0 "$
"
  FileWrite $0 "System Requirements:$
"
  FileWrite $0 "- Windows 10/11 64-bit$
"
  FileWrite $0 "- No additional dependencies required$
"
  FileWrite $0 "$
"
  FileWrite $0 "For support, visit: https://github.com/anixops/easyssh$
"
  FileClose $0

  ; Registry entries
  WriteRegStr HKCU "Software\AnixOps\EasySSHLite" "InstallDir" $INSTDIR
  WriteRegStr HKCU "Software\AnixOps\EasySSHLite" "Version" "${PRODUCT_VERSION}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite" "DisplayName" "EasySSH Lite"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite" "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite" "Publisher" "AnixOps"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite" "HelpLink" "https://github.com/anixops/easyssh"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite" "URLInfoAbout" "https://docs.anixops.com/easyssh-lite"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite" "URLUpdateInfo" "https://github.com/anixops/easyssh/releases"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite" "NoRepair" 1

  ; Get installed size
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite" "EstimatedSize" $0

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd

Section "Start Menu Shortcuts" SecStartMenu
  CreateDirectory "$SMPROGRAMS\EasySSH Lite"
  CreateShortcut "$SMPROGRAMS\EasySSH Lite\EasySSH Lite.lnk" "$INSTDIR\EasySSH Lite.exe" "" "$INSTDIR\icon.ico" 0
  CreateShortcut "$SMPROGRAMS\EasySSH Lite\Uninstall.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\icon.ico" 0
SectionEnd

Section /o "Desktop Shortcut" SecDesktop
  CreateShortcut "$DESKTOP\EasySSH Lite.lnk" "$INSTDIR\EasySSH Lite.exe" "" "$INSTDIR\icon.ico" 0
SectionEnd

;--------------------------------
; Descriptions
;--------------------------------
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecApp} "EasySSH Lite application files (required)"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecStartMenu} "Create Start Menu shortcuts"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} "Create Desktop shortcut"
!insertmacro MUI_FUNCTION_DESCRIPTION_END

;--------------------------------
; Uninstaller Section
;--------------------------------
Section "Uninstall"
  ; Remove application files
  Delete "$INSTDIR\easyssh-lite.exe"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\uninstall.exe"

  ; Remove shortcuts
  Delete "$SMPROGRAMS\EasySSH Lite\EasySSH Lite.lnk"
  Delete "$SMPROGRAMS\EasySSH Lite\Uninstall.lnk"
  RMDir "$SMPROGRAMS\EasySSH Lite"
  Delete "$DESKTOP\EasySSH Lite.lnk"

  ; Remove directories
  RMDir "$INSTDIR"

  ; Remove registry entries
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite"
  DeleteRegKey HKCU "Software\AnixOps\EasySSHLite"
SectionEnd

;--------------------------------
; Initialization
;--------------------------------
Function .onInit
  ; Check Windows version (Windows 10+)
  ${IfNot} ${AtLeastWin10}
    MessageBox MB_OK "This application requires Windows 10 or later."
    Abort
  ${EndIf}

  ; Check for previous installation
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHLite" "UninstallString"
  ${If} $0 != ""
    MessageBox MB_YESNO "A previous version of EasySSH Lite is installed. Do you want to remove it first?" IDNO SkipUninstall
      ExecWait '"$0" /S _?=$INSTDIR'
    SkipUninstall:
  ${EndIf}

  ; Set default section selection
  !insertmacro SelectSection ${SecApp}
  !insertmacro SelectSection ${SecStartMenu}
FunctionEnd

Function un.onInit
  MessageBox MB_YESNO "Are you sure you want to uninstall EasySSH Lite?" IDYES NoAbort
    Abort
  NoAbort:
FunctionEnd
