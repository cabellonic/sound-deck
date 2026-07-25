; Hooks del instalador NSIS.
;
; Desinstalar con el inicio automatico activo dejaba una entrada apuntando a un
; ejecutable que ya no existe, y Windows la sigue intentando en cada inicio de
; sesion. El plugin de autostart escribe en dos lugares: la clave `Run` y el
; registro de estado que usa el Administrador de tareas.
;
; El nombre del valor es el que el plugin toma de `package_info().name`. Se
; borran las dos variantes posibles (nombre de producto y nombre del paquete)
; porque `DeleteRegValue` sobre un valor inexistente no hace nada.

!macro NSIS_HOOK_POSTUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Sound Deck"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "sound-deck"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "Sound Deck"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "sound-deck"
!macroend
