!include "MUI2.nsh"

Name "Ruva Browser"
OutFile "RuvaBrowser-Setup.exe"
InstallDir "$PROGRAMFILES64\RuvaBrowser"
RequestExecutionLevel admin

!define MUI_ICON "installer_files\icon.ico"
!define MUI_UNICON "installer_files\icon.ico"
!define MUI_ABORTWARNING

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "installer_files\LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "Russian"
!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath $INSTDIR
  File "installer_files\ruva-browser.exe"
  File "installer_files\icon.ico"

  ; --- WebView2 runtime check ---
  ; The browser engine is Microsoft Edge WebView2, which ships with
  ; Windows 10/11. If it is missing, run the bundled bootstrapper.
  ReadRegStr $0 HKLM "SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  StrCmp $0 "" 0 webview2_ok
  ReadRegStr $0 HKCU "Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  StrCmp $0 "" 0 webview2_ok

  DetailPrint "Installing Microsoft Edge WebView2 Runtime..."
  File "installer_files\MicrosoftEdgeWebView2Setup.exe"
  ExecWait '"$INSTDIR\MicrosoftEdgeWebView2Setup.exe" /silent /install'
  Delete "$INSTDIR\MicrosoftEdgeWebView2Setup.exe"

webview2_ok:

  CreateDirectory "$SMPROGRAMS\Ruva Browser"
  CreateShortCut "$SMPROGRAMS\Ruva Browser\Ruva Browser.lnk" "$INSTDIR\ruva-browser.exe"
  CreateShortCut "$SMPROGRAMS\Ruva Browser\Uninstall.lnk" "$INSTDIR\uninstall.exe"
  CreateShortCut "$DESKTOP\Ruva Browser.lnk" "$INSTDIR\ruva-browser.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuvaBrowser" "DisplayName" "Ruva Browser"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuvaBrowser" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuvaBrowser" "DisplayIcon" "$\"$INSTDIR\ruva-browser.exe$\""
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuvaBrowser" "Publisher" "Ruva"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\ruva-browser.exe"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Ruva Browser\Ruva Browser.lnk"
  Delete "$SMPROGRAMS\Ruva Browser\Uninstall.lnk"
  RMDir "$SMPROGRAMS\Ruva Browser"
  Delete "$DESKTOP\Ruva Browser.lnk"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuvaBrowser"
SectionEnd
