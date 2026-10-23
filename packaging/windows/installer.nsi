Unicode true
!include "MUI2.nsh"

!ifndef VERSION
  !define VERSION "0.0.0"
!endif
!ifndef ARCH
  !define ARCH "x86_64"
!endif

!define UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\ScreenScan"

Name "ScreenScan"
Caption "ScreenScan ${VERSION} 安装程序"
OutFile "ScreenScan-${VERSION}-windows-${ARCH}-setup.exe"
InstallDir "$LOCALAPPDATA\Programs\ScreenScan"
InstallDirRegKey HKCU "Software\ScreenScan" "InstallDir"
RequestExecutionLevel user
SetCompressor /SOLID lzma

!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "SimpChinese"
!insertmacro MUI_LANGUAGE "English"

Section "ScreenScan" SecMain
  SetShellVarContext current
  SetOutPath "$INSTDIR"
  File "ScreenScan.exe"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  WriteRegStr HKCU "Software\ScreenScan" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayName" "ScreenScan"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINST_KEY}" "Publisher" "Spinach5"
  WriteRegStr HKCU "${UNINST_KEY}" "DisplayIcon" "$INSTDIR\ScreenScan.exe"
  WriteRegStr HKCU "${UNINST_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST_KEY}" "NoRepair" 1

  CreateDirectory "$SMPROGRAMS\ScreenScan"
  CreateShortcut "$SMPROGRAMS\ScreenScan\ScreenScan.lnk" "$INSTDIR\ScreenScan.exe"
  CreateShortcut "$SMPROGRAMS\ScreenScan\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\ScreenScan.lnk" "$INSTDIR\ScreenScan.exe"
SectionEnd

Section "Uninstall"
  SetShellVarContext current
  Delete "$INSTDIR\ScreenScan.exe"
  Delete "$INSTDIR\Uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\ScreenScan\ScreenScan.lnk"
  Delete "$SMPROGRAMS\ScreenScan\Uninstall.lnk"
  RMDir "$SMPROGRAMS\ScreenScan"
  Delete "$DESKTOP\ScreenScan.lnk"

  DeleteRegKey HKCU "${UNINST_KEY}"
  DeleteRegKey HKCU "Software\ScreenScan"
SectionEnd
