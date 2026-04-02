; EasySSH Pro NSIS Installer Script
; Enterprise SSH client with team collaboration
;
; Build command:
;   makensis /DPRODUCT_VERSION=0.3.0 easyssh-pro.nsi

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

Name "EasySSH Pro"
OutFile "EasySSH-Pro-${PRODUCT_VERSION}-x64.exe"
InstallDir "$PROGRAMFILES64\AnixOps\EasySSH Pro"
InstallDirRegKey HKLM "Software\AnixOps\EasySSHPro" "InstallDir"
RequestExecutionLevel admin

;--------------------------------
; Version Information
;--------------------------------
VIProductVersion "${PRODUCT_VERSION}.0"
VIAddVersionKey "ProductName" "EasySSH Pro"
VIAddVersionKey "CompanyName" "AnixOps"
VIAddVersionKey "LegalCopyright" "Copyright (c) 2026 AnixOps"
VIAddVersionKey "FileDescription" "EasySSH Pro - Enterprise SSH Client"
VIAddVersionKey "FileVersion" "${PRODUCT_VERSION}"
VIAddVersionKey "ProductVersion" "${PRODUCT_VERSION}"
VIAddVersionKey "InternalName" "EasySSHPro"
VIAddVersionKey "OriginalFilename" "EasySSH-Pro.exe"

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
!define MUI_FINISHPAGE_RUN "$INSTDIR\EasySSH Pro.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch EasySSH Pro"
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
Section "EasySSH Pro (Required)" SecApp
  SectionIn RO

  SetOutPath "$INSTDIR"

  ; Main executable
  File "..\..\..\..\target\release-pro\easyssh-pro.exe"
  File "..\..\..\..\crates\easyssh-platforms\windows\easyssh-winui\assets\icon.ico"
  File "..\..\..\..\LICENSE"

  ; WebView2 loader
  File "..\..\..\..\target\release-pro\WebView2Loader.dll"

  ; Create README
  FileOpen $0 "$INSTDIR\README.txt" w
  FileWrite $0 "EasySSH Pro v${PRODUCT_VERSION}$
"
  FileWrite $0 "=================$
"
  FileWrite $0 "$
"
  FileWrite $0 "Quick Start:$
"
  FileWrite $0 "1. Run EasySSH Pro.exe$
"
  FileWrite $0 "2. Sign in with your team credentials$
"
  FileWrite $0 "3. Access shared servers and snippets$
"
  FileWrite $0 "4. Collaborate with your team$
"
  FileWrite $0 "$
"
  FileWrite $0 "System Requirements:$
"
  FileWrite $0 "- Windows 10/11 64-bit$
"
  FileWrite $0 "- Microsoft Edge WebView2 Runtime$
"
  FileWrite $0 "- Internet connection for team features$
"
  FileWrite $0 "$
"
  FileWrite $0 "For support, visit: https://github.com/anixops/easyssh$
"
  FileClose $0

  ; Resources folder
  SetOutPath "$INSTDIR\resources"
  File /r "..\..\..\..\target\release-pro\resources\*"

  ; Pro server (local mode)
  SetOutPath "$INSTDIR\server"
  File "..\..\..\..\target\release-pro\server\easyssh-pro-server.exe"
  File "..\..\..\..\target\release-pro\server\config.yaml"

  ; Registry entries
  WriteRegStr HKLM "Software\AnixOps\EasySSHPro" "InstallDir" $INSTDIR
  WriteRegStr HKLM "Software\AnixOps\EasySSHPro" "Version" "${PRODUCT_VERSION}"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro" "DisplayName" "EasySSH Pro"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro" "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro" "Publisher" "AnixOps"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro" "HelpLink" "https://github.com/anixops/easyssh"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro" "URLInfoAbout" "https://docs.anixops.com/easyssh-pro"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro" "URLUpdateInfo" "https://github.com/anixops/easyssh/releases"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro" "NoModify" 1
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro" "NoRepair" 1

  ; Get installed size
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro" "EstimatedSize" $0

  ; Create uninstaller
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Add firewall exception for Pro server
  nsExec::Exec 'netsh advfirewall firewall add rule name="EasySSH Pro Server" dir=in action=allow program="$INSTDIR\server\easyssh-pro-server.exe" enable=yes'
SectionEnd

Section "Start Menu Shortcuts" SecStartMenu
  CreateDirectory "$SMPROGRAMS\EasySSH Pro"
  CreateShortcut "$SMPROGRAMS\EasySSH Pro\EasySSH Pro.lnk" "$INSTDIR\EasySSH Pro.exe" "" "$INSTDIR\icon.ico" 0
  CreateShortcut "$SMPROGRAMS\EasySSH Pro\Uninstall.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\icon.ico" 0
SectionEnd

Section /o "Desktop Shortcut" SecDesktop
  CreateShortcut "$DESKTOP\EasySSH Pro.lnk" "$INSTDIR\EasySSH Pro.exe" "" "$INSTDIR\icon.ico" 0
SectionEnd

Section /o "Pro Server (Local Mode)" SecServer
  ; Server is already installed, this just controls the firewall rule
SectionEnd

;--------------------------------
; Descriptions
;--------------------------------
!insertmacro MUI_FUNCTION_DESCRIPTION_BEGIN
  !insertmacro MUI_DESCRIPTION_TEXT ${SecApp} "EasySSH Pro application files (required)"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecStartMenu} "Create Start Menu shortcuts"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecDesktop} "Create Desktop shortcut"
  !insertmacro MUI_DESCRIPTION_TEXT ${SecServer} "Install local Pro Server for offline team mode"
!insertmacro MUI_FUNCTION_DESCRIPTION_END

;--------------------------------
; Uninstaller Section
;--------------------------------
Section "Uninstall"
  ; Remove firewall rule
  nsExec::Exec 'netsh advfirewall firewall delete rule name="EasySSH Pro Server"'

  ; Remove application files
  Delete "$INSTDIR\easyssh-pro.exe"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\README.txt"
  Delete "$INSTDIR\WebView2Loader.dll"
  Delete "$INSTDIR\uninstall.exe"

  ; Remove resources
  RMDir /r "$INSTDIR\resources"

  ; Remove server
  RMDir /r "$INSTDIR\server"

  ; Remove shortcuts
  Delete "$SMPROGRAMS\EasySSH Pro\EasySSH Pro.lnk"
  Delete "$SMPROGRAMS\EasySSH Pro\Uninstall.lnk"
  RMDir "$SMPROGRAMS\EasySSH Pro"
  Delete "$DESKTOP\EasySSH Pro.lnk"

  ; Remove directories
  RMDir "$INSTDIR"

  ; Remove registry entries
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro"
  DeleteRegKey HKLM "Software\AnixOps\EasySSHPro"
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
  ReadRegStr $0 HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\EasySSHPro" "UninstallString"
  ${If} $0 != ""
    MessageBox MB_YESNO "A previous version of EasySSH Pro is installed. Do you want to remove it first?" IDNO SkipUninstall
      ExecWait '"$0" /S _?=$INSTDIR'
    SkipUninstall:
  ${EndIf}

  ; Set default section selection
  !insertmacro SelectSection ${SecApp}
  !insertmacro SelectSection ${SecStartMenu}
  !insertmacro SelectSection ${SecServer}
FunctionEnd

Function un.onInit
  MessageBox MB_YESNO "Are you sure you want to uninstall EasySSH Pro?" IDYES NoAbort
    Abort
  NoAbort:
FunctionEnd
