; EasySSH Standard NSIS Installer Script
; Full-featured SSH client with embedded terminal
;
; Build command:
;   makensis /DPRODUCT_VERSION=0.3.0 easyssh-standard.nsi

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

Name "EasySSH Standard"
OutFile "EasySSH-Standard-${PRODUCT_VERSION}-x64.exe"
InstallDir "$PROGRAMFILES64\AnixOps\EasySSH Standard"
InstallDirRegKey HKLM "Software\AnixOps\EasySSHStandard" "InstallDir"
RequestExecutionLevel admin

;--------------------------------
; Version Information
;--------------------------------
VIProductVersion "${PRODUCT_VERSION}.0"
VIAddVersionKey "ProductName" "EasySSH Standard"
VIAddVersionKey "CompanyName" "AnixOps"
VIAddVersionKey "LegalCopyright" "Copyright (c) 2026 AnixOps"
VIAddVersionKey "FileDescription" "EasySSH Standard - Full-Featured SSH Client"
VIAddVersionKey "FileVersion" "${PRODUCT_VERSION}"
VIAddVersionKey "ProductVersion" "${PRODUCT_VERSION}"
VIAddVersionKey "InternalName" "EasySSHStandard"
VIAddVersionKey "OriginalFilename" "EasySSH-Standard.exe"

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
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_NOAUTOCLOSE
!define MUI_FINISHPAGE_RUN "$INSTDIR\EasySSH Standard.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch EasySSH Standard"
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
Section "EasySSH Standard (Required)" SecApp
  SectionIn RO

  SetOutPath "$INSTDIR"

  ; Main executable
  File "..\..\..\..\target\release-standard\easyssh-standard.exe"
  File "..\..\..\..\crates\easyssh-platforms\windows\easyssh-winui\assets\icon.ico"
  File "..\..\..\..\LICENSE"

  ; WebView2 loader
  File "..\..\..\..\target\release-standard\WebView2Loader.dll"

  ; Create README
  FileOpen $0 "$INSTDIR\README.txt" w
  FileWrite $0 "EasySSH Standard v${PRODUCT_VERSION}$
"
  FileWrite $0 "=====================$
"
  FileWrite $0 "$
"
  FileWrite $0 "Quick Start:$
"
  FileWrite $0 "1. Run EasySSH Standard.exe$
"
  FileWrite $0 "2. Add your SSH servers via the UI$
"
  FileWrite $0 "3. Connect with embedded terminal$
"
  FileWrite $0 "4. Use split-screen for multiple sessions$
"
  FileWrite $0 "$
"
  FileWrite $0 "System Requirements:$
"
  FileWrite $0 "- Windows 10/11 64-bit$
"
  FileWrite $0 "- Microsoft Edge WebView2 Runtime$
"
  FileWrite $0 "$
"
  FileWrite $0 "For support, visit: https://github.com/anixops/easyssh$
"
  FileClose $0

  ; Resources folder
  SetOutPath "$INSTDIR\resources"
  File /r "..\..\..\..\target\release-standard\resources\*"

  ; Registry entries
  WriteRegStr HKLM "Software\AnixOps\EasySSHStandard" "InstallDir" $INSTDIR
  WriteRegStr HKLM "Software\AnixOps\EasySSHStandard" "Version" "${PRODUCT_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard" "DisplayName" "EasySSH Standard"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard" "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard" "Publisher" "AnixOps"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard" "HelpLink" "https://github.com/anixops/easyssh"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard" "URLInfoAbout" "https://docs.anixops.com/easyssh-standard"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard" "URLUpdateInfo" "https://github.com/anixops/easyssh/releases"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard" "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard" "NoRepair" 1

  ; Get installed size
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard" "EstimatedSize" $0

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"
SectionEnd

Section "Start Menu Shortcuts" SecStartMenu
  CreateDirectory "$SMPROGRAMS\EasySSH Standard"
  CreateShortcut "$SMPROGRAMS\EasySSH Standard\EasySSH Standard.lnk" "$INSTDIR\EasySSH Standard.exe" "" "$INSTDIR\icon.ico" 0
  CreateShortcut "$SMPROGRAMS\EasySSH Standard\Uninstall.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\icon.ico" 0
SectionEnd

Section /o "Desktop Shortcut" SecDesktop
  CreateShortcut "$DESKTOP\EasySSH Standard.lnk" "$INSTDIR\EasySSH Standard.exe" "" "$INSTDIR\icon.ico" 0
SectionEnd

;--------------------------------
; Descriptions
;--------------------------------
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecApp} "EasySSH Standard application files (required)"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecStartMenu} "Create Start Menu shortcuts"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} "Create Desktop shortcut"
!insertmacro MUI_FUNCTION_DESCRIPTION_END

;--------------------------------
; Uninstaller Section
;--------------------------------
Section "Uninstall"
  ; Remove application files
  Delete "$INSTDIR\easyssh-standard.exe"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\WebView2Loader.dll"
  Delete "$INSTDIR\uninstall.exe"

  ; Remove resources
  RMDir /r "$INSTDIR\resources"

  ; Remove shortcuts
  Delete "$SMPROGRAMS\EasySSH Standard\EasySSH Standard.lnk"
  Delete "$SMPROGRAMS\EasySSH Standard\Uninstall.lnk"
  RMDir "$SMPROGRAMS\EasySSH Standard"
  Delete "$DESKTOP\EasySSH Standard.lnk"

  ; Remove directories
  RMDir "$INSTDIR"

  ; Remove registry entries
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard"
  DeleteRegKey HKLM "Software\AnixOps\EasySSHStandard"
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

  ; Check for WebView2 Runtime
  ReadRegStr $0 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  ${If} $0 == ""
    MessageBox MB_YESNO "Microsoft Edge WebView2 Runtime is required but not detected. Download and install now?" IDNO SkipWebView2
      ExecShell "open" "https://go.microsoft.com/fwlink/p/?LinkId=2124703"
    SkipWebView2:
  ${EndIf}

  ; Check for previous installation
  ReadRegStr $0 HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHStandard" "UninstallString"
  ${If} $0 != ""
    MessageBox MB_YESNO "A previous version of EasySSH Standard is installed. Do you want to remove it first?" IDNO SkipUninstall
      ExecWait '"$0" /S _?=$INSTDIR'
    SkipUninstall:
  ${EndIf}

  ; Set default section selection
  !insertmacro SelectSection ${SecApp}
  !insertmacro SelectSection ${SecStartMenu}
FunctionEnd

Function un.onInit
  MessageBox MB_YESNO "Are you sure you want to uninstall EasySSH Standard?" IDYES NoAbort
    Abort
  NoAbort:
FunctionEnd
