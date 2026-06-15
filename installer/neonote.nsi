Unicode true
ManifestDPIAware true
RequestExecutionLevel user

!include "MUI2.nsh"

!ifndef APP_NAME
  !define APP_NAME "NeoNote"
!endif

!ifndef APP_VERSION
  !define APP_VERSION "1.0.0"
!endif

!ifndef SOURCE_EXE
  !error "SOURCE_EXE must be defined"
!endif

!ifndef SOURCE_THEME_DIR
  !error "SOURCE_THEME_DIR must be defined"
!endif

!ifndef OUTPUT_DIR
  !error "OUTPUT_DIR must be defined"
!endif

!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${APP_NAME}"
!define INSTALL_ROOT_KEY "Software\${APP_NAME}"

Name "${APP_NAME}"
OutFile "${OUTPUT_DIR}\${APP_NAME}-Setup-${APP_VERSION}.exe"
InstallDir "$LOCALAPPDATA\Programs\${APP_NAME}"
InstallDirRegKey HKCU "${INSTALL_ROOT_KEY}" "InstallDir"
BrandingText "${APP_NAME} ${APP_VERSION}"

!define MUI_ABORTWARNING
!define MUI_ICON "${NSISDIR}\Contrib\Graphics\Icons\modern-install.ico"
!define MUI_UNICON "${NSISDIR}\Contrib\Graphics\Icons\modern-uninstall.ico"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!define MUI_FINISHPAGE_RUN "$INSTDIR\NeoNote.exe"
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Section "Install"
  SetOutPath "$INSTDIR"
  File "/oname=NeoNote.exe" "${SOURCE_EXE}"

  SetOutPath "$INSTDIR\assets\themes\built-in"
  File "${SOURCE_THEME_DIR}\*.json"

  WriteUninstaller "$INSTDIR\Uninstall.exe"

  WriteRegStr HKCU "${INSTALL_ROOT_KEY}" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "${APP_NAME}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${APP_VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "NeoNote"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\NeoNote.exe"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1

  CreateDirectory "$SMPROGRAMS\${APP_NAME}"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk" "$INSTDIR\NeoNote.exe"
  CreateShortcut "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk" "$INSTDIR\Uninstall.exe"
  CreateShortcut "$DESKTOP\${APP_NAME}.lnk" "$INSTDIR\NeoNote.exe"
SectionEnd

Section "Uninstall"
  ExecWait '"$INSTDIR\NeoNote.exe" "--unregister-file-associations"'

  Delete "$DESKTOP\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\${APP_NAME}.lnk"
  Delete "$SMPROGRAMS\${APP_NAME}\Uninstall ${APP_NAME}.lnk"
  RMDir "$SMPROGRAMS\${APP_NAME}"

  Delete "$INSTDIR\NeoNote.exe"
  Delete "$INSTDIR\Uninstall.exe"
  Delete "$INSTDIR\assets\themes\built-in\*.json"
  RMDir "$INSTDIR\assets\themes\built-in"
  RMDir "$INSTDIR\assets\themes"
  RMDir "$INSTDIR\assets"
  RMDir "$INSTDIR"

  DeleteRegKey HKCU "${UNINSTALL_KEY}"
  DeleteRegKey HKCU "${INSTALL_ROOT_KEY}"
SectionEnd
