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

Section "WebView2 Runtime"
  DetailPrint "Checking WebView2 Runtime..."
  IfFileExists "$LOCALAPPDATA\Microsoft\EdgeWebView\EBWebView\*\msedge_webview2.exe" skip_webview2
    DetailPrint "Installing WebView2 Runtime..."
    SetOutPath "$TEMP\webview2"
    File "installer_files\MicrosoftEdgeWebview2Setup.exe"
    ExecWait '"$TEMP\webview2\MicrosoftEdgeWebview2Setup.exe" /silent /install' $0
    DetailPrint "WebView2 install result: $0"
    RMDir /r "$TEMP\webview2"
  skip_webview2:
    DetailPrint "WebView2 Ready"
SectionEnd

Section "Install"
  SetOutPath $INSTDIR
  File "installer_files\ruva-browser.exe"
  File "installer_files\WebView2Loader.dll"

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
  Delete "$INSTDIR\WebView2Loader.dll"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Ruva Browser\Ruva Browser.lnk"
  Delete "$SMPROGRAMS\Ruva Browser\Uninstall.lnk"
  RMDir "$SMPROGRAMS\Ruva Browser"
  Delete "$DESKTOP\Ruva Browser.lnk"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuvaBrowser"
SectionEnd
