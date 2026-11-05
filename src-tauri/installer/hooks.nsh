; Update the per-user PATH through the registry: [Environment]::GetEnvironmentVariable
; returns expanded values, and writing those back would permanently replace entries
; such as %JAVA_HOME%\bin with literal paths. The install directory is passed through
; the environment because it may contain characters that are special to PowerShell.
!macro ORCASVN_UPDATE_USER_PATH SCRIPT
  System::Call 'kernel32::SetEnvironmentVariable(t "ORCASVN_INSTDIR", t "$INSTDIR")'
  nsExec::Exec `powershell -NoProfile -ExecutionPolicy Bypass -Command "$$d=$$env:ORCASVN_INSTDIR; $$k=[Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment'); $$p=$$k.GetValue('Path','',[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames); $$parts=@($$p -split ';' | Where-Object { $$_ -and $$_.TrimEnd('\') -ne $$d.TrimEnd('\') }); ${SCRIPT}; $$k.Close()"`
  ; nsExec leaves its result on the stack.
  Exch $0
  StrCmp $0 "0" +2
  DetailPrint "Could not update the user PATH: $0"
  Pop $0
  ; 0xFFFF is HWND_BROADCAST and 0x1A is WM_SETTINGCHANGE; a registry write does not
  ; notify other processes, and new terminals inherit their environment from Explorer.
  SendMessage 0xFFFF 0x1A 0 "STR:Environment" /TIMEOUT=5000
!macroend

; Register the install directory in the per-user PATH so `orcasvn` works from a new terminal.
!macro NSIS_HOOK_POSTINSTALL
  !insertmacro ORCASVN_UPDATE_USER_PATH "$$k.SetValue('Path',((@($$parts)+$$d) -join ';'),[Microsoft.Win32.RegistryValueKind]::ExpandString)"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  !insertmacro ORCASVN_UPDATE_USER_PATH "if ($$parts.Count) { $$k.SetValue('Path',($$parts -join ';'),[Microsoft.Win32.RegistryValueKind]::ExpandString) } else { $$k.DeleteValue('Path',$$false) }"
!macroend
