!include "MUI2.nsh"

Name "Ruva Brower"
OutFile "RuvaBrowser-Setup.exe"
InstallDir "$PROGRAMFILES64\RuvaBrower"
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

  CreateDirectory "$SMPROGRAMS\Ruva Brower"
  CreateShortCut "$SMPROGRAMS\Ruva Brower\Ruva Brower.lnk" "$INSTDIR\ruva-browser.exe"
  CreateShortCut "$SMPROGRAMS\Ruva Brower\Uninstall.lnk" "$INSTDIR\uninstall.exe"
  CreateShortCut "$DESKTOP\Ruva Brower.lnk" "$INSTDIR\ruva-browser.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuvaBrower" "DisplayName" "Ruva Brower"
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuvaBrower" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuvaBrower" "DisplayIcon" "$\"$INSTDIR\ruva-browser.exe$\""
  WriteRegStr HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuvaBrower" "Publisher" "Ruva"
SectionEnd

Section "Uninstall"
  Delete "$INSTDIR\ruva-browser.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\Ruva Brower\Ruva Brower.lnk"
  Delete "$SMPROGRAMS\Ruva Brower\Uninstall.lnk"
  RMDir "$SMPROGRAMS\Ruva Brower"
  Delete "$DESKTOP\Ruva Brower.lnk"
  DeleteRegKey HKLM "Software\Microsoft\Windows\CurrentVersion\Uninstall\RuvaBrower"
SectionEnd
