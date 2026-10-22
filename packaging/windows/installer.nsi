; ============================================================
;  DocCode — установщик для Windows (NSIS, x64)
;
;  Собирается packaging/windows/build-installer.ps1. Он же:
;    * выставляет рабочий каталог = packaging/windows,
;      поэтому относительные пути ниже читаются отсюда;
;    * передаёт версию и размер файла: makensis /D... /D...
;
;  Вручную:
;    cd packaging\windows
;    makensis /DPRODUCT_VERSION=<ver> /DESTIMATED_SIZE=19234 installer.nsi
;
;  Скрипт намеренно ASCII-only: локализацию страниц дают встроенные
;  переводы MUI_LANGUAGE, поэтому UTF-8 BOM здесь не требуется.
; ============================================================

!ifndef PRODUCT_VERSION
  !error "PRODUCT_VERSION не задан. Используйте packaging/windows/build-installer.ps1 или makensis /DPRODUCT_VERSION=<ver> — версия берётся из Cargo.toml."
!endif

!define PRODUCT_NAME    "DocCode"
!define PRODUCT_EXE     "doccode.exe"
!define PRODUCT_PUB     "DocCode"
!define PRODUCT_WEB     "https://github.com/doccode/doccode"
!define PRODUCT_REG_KEY "Software\DocCode"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\DocCode"

Unicode true
SetCompressor /SOLID lzma

; --- Версия в свойствах setup.exe -------------------------------------------
; SignPath проверяет product-name/product-version (metadata restrictions),
; поэтому без этих ключей подпись не подтвердится. PRODUCT_VERSION4 передаёт
; build-installer.ps1: NSIS требует здесь ровно четыре числа.
!ifndef PRODUCT_VERSION4
  !define PRODUCT_VERSION4 "${PRODUCT_VERSION}.0"
!endif
VIProductVersion "${PRODUCT_VERSION4}"
VIAddVersionKey /LANG=1033 "ProductName"      "${PRODUCT_NAME}"
VIAddVersionKey /LANG=1033 "ProductVersion"   "${PRODUCT_VERSION}"
VIAddVersionKey /LANG=1033 "FileVersion"      "${PRODUCT_VERSION}"
VIAddVersionKey /LANG=1033 "FileDescription"  "${PRODUCT_NAME} installer"
VIAddVersionKey /LANG=1033 "LegalCopyright"   "Copyright (c) DocCode"
VIAddVersionKey /LANG=1033 "OriginalFilename" "DocCode-${PRODUCT_VERSION}-x64-setup.exe"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "DocCode-${PRODUCT_VERSION}-x64-setup.exe"
InstallDir "$PROGRAMFILES64\${PRODUCT_NAME}"
InstallDirRegKey HKLM "${PRODUCT_REG_KEY}" "InstallDir"
RequestExecutionLevel admin
CRCCheck on
ShowInstDetails nevershow
ShowUninstDetails nevershow

!include "MUI2.nsh"
!include "x64.nsh"

; --- Иконки установщика/деинсталлятора -------------------------------------
!define MUI_ICON "..\icon\doccode.ico"
!define MUI_UNICON "..\icon\doccode.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\${PRODUCT_EXE}"
!define MUI_FINISHPAGE_RUN_TEXT "Launch ${PRODUCT_NAME}"

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_COMPONENTS
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_UNPAGE_FINISH

!insertmacro MUI_LANGUAGE "English"
!insertmacro MUI_LANGUAGE "Russian"

; ============================================================
;  Компоненты
; ============================================================

Section "${PRODUCT_NAME} (main files)" SecMain
  SectionIn RO
  SetShellVarContext all
  SetRegView 64

  SetOutPath "$INSTDIR"
  SetOverwrite on
  File "..\..\target\release\doccode.exe"
  File "..\..\prompts.yaml"
  File "..\icon\doccode.ico"

  ; Деинсталлятор пишем последним — файлы выше всегда успешны.
  WriteUninstaller "$INSTDIR\uninstall.exe"

  ; Ярлыки в меню «Пуск» (для всех пользователей).
  CreateDirectory "$SMPROGRAMS\${PRODUCT_NAME}"
  CreateShortCut "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk" "$INSTDIR\${PRODUCT_EXE}"
  CreateShortCut "$SMPROGRAMS\${PRODUCT_NAME}\Uninstall ${PRODUCT_NAME}.lnk" "$INSTDIR\uninstall.exe"

  ; Данные для «Установка и удаление программ».
  WriteRegStr HKLM "${PRODUCT_REG_KEY}" "InstallDir" "$INSTDIR"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayName" "${PRODUCT_NAME} ${PRODUCT_VERSION}"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayIcon" "$INSTDIR\${PRODUCT_EXE}"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUB}"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB}"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "UninstallString" "$\"$INSTDIR\uninstall.exe$\""
  WriteRegStr HKLM "${PRODUCT_UNINST_KEY}" "QuietUninstallString" "$\"$INSTDIR\uninstall.exe$\" /S"
  WriteRegDWORD HKLM "${PRODUCT_UNINST_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${PRODUCT_UNINST_KEY}" "NoRepair" 1
!ifdef ESTIMATED_SIZE
  WriteRegDWORD HKLM "${PRODUCT_UNINST_KEY}" "EstimatedSize" ${ESTIMATED_SIZE}
!endif
SectionEnd

Section "Desktop shortcut" SecDesktop
  SetShellVarContext all
  CreateShortCut "$DESKTOP\${PRODUCT_NAME}.lnk" "$INSTDIR\${PRODUCT_EXE}"
SectionEnd

; ============================================================
;  Деинсталляция
; ============================================================

Section "Uninstall"
  SetShellVarContext all
  SetRegView 64

  Delete "$INSTDIR\doccode.exe"
  Delete "$INSTDIR\prompts.yaml"
  Delete "$INSTDIR\doccode.ico"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"

  Delete "$SMPROGRAMS\${PRODUCT_NAME}\${PRODUCT_NAME}.lnk"
  Delete "$SMPROGRAMS\${PRODUCT_NAME}\Uninstall ${PRODUCT_NAME}.lnk"
  RMDir "$SMPROGRAMS\${PRODUCT_NAME}"
  Delete "$DESKTOP\${PRODUCT_NAME}.lnk"

  DeleteRegKey HKLM "${PRODUCT_UNINST_KEY}"
  DeleteRegKey HKLM "${PRODUCT_REG_KEY}"
SectionEnd

; ============================================================
;  Инициализация
; ============================================================

Function .onInit
  SetRegView 64
  ${IfNot} ${RunningX64}
    MessageBox MB_OK|MB_ICONSTOP \
      "${PRODUCT_NAME} requires 64-bit Windows (x86_64).$\r$\n\
      DocCode требует 64-битной версии Windows."
    Abort
  ${EndIf}
FunctionEnd

Function un.onInit
  SetShellVarContext all
  SetRegView 64
FunctionEnd
