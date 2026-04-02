; EasySSH NSIS Installer Script
; Creates a professional Windows installer
;
; Build command:
;   makensis easyssh.nsi
;
; Requirements:
;   - NSIS 3.0+
;   - Access to WiX toolset (for optional MSI generation)

!include "MUI2.nsh"
!include "LogicLib.nsh"
!include "FileFunc.nsh"
!include "WinVer.nsh"

;--------------------------------
; General Configuration
;--------------------------------
Name "EasySSH"
OutFile "EasySSH-0.3.0-x64.exe"
InstallDir "$LOCALAPPDATA\Programs\EasySSH"
InstallDirRegKey HKCU "Software\AnixOps\EasySSH" "InstallDir"
RequestExecutionLevel user

;--------------------------------
; Version Information
;--------------------------------
VIProductVersion "0.3.0.0"
VIAddVersionKey "ProductName" "EasySSH"
VIAddVersionKey "CompanyName" "AnixOps"
VIAddVersionKey "LegalCopyright" "Copyright (c) 2026 AnixOps"
VIAddVersionKey "FileDescription" "EasySSH - Native SSH Client for Windows"
VIAddVersionKey "FileVersion" "0.3.0"
VIAddVersionKey "ProductVersion" "0.3.0"
VIAddVersionKey "InternalName" "EasySSH"
VIAddVersionKey "OriginalFilename" "EasySSH.exe"

;--------------------------------
; MUI Configuration
;--------------------------------
!define MUI_ICON "..\..\core\icons\icon.ico"
!define MUI_UNICON "..\..\core\icons\icon.ico"
!define MUI_HEADERIMAGE
!define MUI_HEADERIMAGE_BITMAP "images\header.bmp"
!define MUI_WELCOMEFINISHPAGE_BITMAP "images\welcome.bmp"
!define MUI_UNWELCOMEFINISHPAGE_BITMAP "images\welcome.bmp"

;--------------------------------
; Pages
;--------------------------------
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "..\..\LICENSE"
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_NOAUTOCLOSE
!define MUI_FINISHPAGE_RUN "$INSTDIR\EasySSH.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch EasySSH"
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
Section "EasySSH (Required)" SecApp
  SectionIn RO

  SetOutPath "$INSTDIR"

  ; Main executable
  File "..\..\target\release\EasySSH.exe"

  ; Resources
  File "..\..\core\icons\icon.ico"
  File "..\..\LICENSE"

  ; Documentation
  FileOpen $0 "$INSTDIR\README.txt" w
  FileWrite $0 "EasySSH v0.3.0$
"
  FileWrite $0 "=================$
"
  FileWrite $0 "$
"
  FileWrite $0 "Quick Start:$
"
  FileWrite $0 "1. Run EasySSH.exe$
"
  FileWrite $0 "2. Add your SSH servers via the UI$
"
  FileWrite $0 "3. Connect using password or key authentication$
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
  WriteRegStr HKCU "Software\AnixOps\EasySSH" "InstallDir" $INSTDIR
  WriteRegStr HKCU "Software\AnixOps\EasySSH" "Version" "0.3.0"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "DisplayName" "EasySSH"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "Publisher" "AnixOps"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "HelpLink" "https://github.com/anixops/easyssh"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "URLInfoAbout" "https://docs.anixops.com/easyssh"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "URLUpdateInfo" "https://github.com/anixops/easyssh/releases"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "DisplayVersion" "0.3.0"
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "VersionMajor" 0
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "VersionMinor" 3
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "NoRepair" 1

  ; Get installed size
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "EstimatedSize" $0

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd

Section "Start Menu Shortcuts" SecStartMenu
  CreateDirectory "$SMPROGRAMS\EasySSH"
  CreateShortcut "$SMPROGRAMS\EasySSH\EasySSH.lnk" "$INSTDIR\EasySSH.exe" "" "$INSTDIR\icon.ico" 0
  CreateShortcut "$SMPROGRAMS\EasySSH\Uninstall.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\icon.ico" 0
SectionEnd

Section /o "Desktop Shortcut" SecDesktop
  CreateShortcut "$DESKTOP\EasySSH.lnk" "$INSTDIR\EasySSH.exe" "" "$INSTDIR\icon.ico" 0
SectionEnd

;--------------------------------
; Descriptions
;--------------------------------
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecApp} "EasySSH application files (required)"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecStartMenu} "Create Start Menu shortcuts"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} "Create Desktop shortcut"
!insertmacro MUI_FUNCTION_DESCRIPTION_END

;--------------------------------
; Uninstaller Section
;--------------------------------
Section "Uninstall"
  ; Remove application files
  Delete "$INSTDIR\EasySSH.exe"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\uninstall.exe"

  ; Remove shortcuts
  Delete "$SMPROGRAMS\EasySSH\EasySSH.lnk"
  Delete "$SMPROGRAMS\EasySSH\Uninstall.lnk"
  RMDir "$SMPROGRAMS\EasySSH"
  Delete "$DESKTOP\EasySSH.lnk"

  ; Remove directories
  RMDir "$INSTDIR"

  ; Remove registry entries
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH"
  DeleteRegKey HKCU "Software\AnixOps\EasySSH"
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
  ReadRegStr $0 HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSH" "UninstallString"
  ${If} $0 != ""
    MessageBox MB_YESNO "A previous version of EasySSH is installed. Do you want to remove it first?" IDNO SkipUninstall
      ExecWait '"$0" /S _?=$INSTDIR'
    SkipUninstall:
  ${EndIf}

  ; Set default section selection
  !insertmacro SelectSection ${SecApp}
  !insertmacro SelectSection ${SecStartMenu}
FunctionEnd

Function un.onInit
  MessageBox MB_YESNO "Are you sure you want to uninstall EasySSH?" IDYES NoAbort
    Abort
  NoAbort:
FunctionEnd

;--------------------------------
; Helper Functions
;--------------------------------
Function .onVerifyInstDir
  ; Verify installation directory is not in use
  ${If} ${FileExists} "$INSTDIR\EasySSH.exe"
    MessageBox MB_YESNO "EasySSH is currently running. Please close it before continuing." IDYES Continue
      Abort
    Continue:
  ${EndIf}
FunctionEnd
