<#
.SYNOPSIS
  Compila el paquete Linux de Sound Deck dentro de WSL y deja los artefactos
  (.deb + binario) en dist-linux/, junto al .exe de Windows.

.DESCRIPTION
  Todo ocurre en la maquina local: no hace falta ningun servidor remoto ni
  Docker. La primera corrida instala la toolchain dentro de la distro de WSL
  (Rust, Node 22, pnpm y las dependencias de sistema de Tauri) y tarda varios
  minutos; las siguientes reutilizan esa instalacion, el cache de cargo y los
  node_modules que quedan del lado de WSL.

  El codigo se copia al sistema de archivos nativo de la distro antes de
  compilar. Compilar directamente sobre /mnt/c funciona, pero es varias veces
  mas lento: cada acceso a un archivo cruza el puente hacia NTFS.

.PARAMETER Distro
  Distribucion de WSL a usar. Por defecto la predeterminada del sistema.

.PARAMETER SkipWindowsBuild
  No recompila el .exe de Windows; usa el que ya exista en target/release.

.PARAMETER LinuxOnly
  Genera solo los artefactos de Linux, sin tocar ni copiar el .exe.

.EXAMPLE
  pnpm build:linux
  powershell -File scripts/build-linux.ps1 -Distro Ubuntu-24.04 -LinuxOnly
#>
[CmdletBinding()]
param(
  [string]$Distro,
  [switch]$SkipWindowsBuild,
  [switch]$LinuxOnly
)

$ErrorActionPreference = 'Stop'
$ProjectRoot = Split-Path -Parent $PSScriptRoot
Set-Location $ProjectRoot

$LocalOut = Join-Path $ProjectRoot 'dist-linux'
$Exe = Join-Path $ProjectRoot 'src-tauri/target/release/sound-deck.exe'

# `wsl.exe -d <distro>` solo si se pidio una; si no, la predeterminada.
$DistroArgs = if ($Distro) { @('-d', $Distro) } else { @() }

function Invoke-Wsl {
  param([Parameter(Mandatory)][string[]]$Arguments, [string]$FailureMessage)

  # Siempre como root: la distro es un entorno de compilacion descartable, y
  # asi apt no pide contrasena ni hay permisos cruzados con el cache de cargo.
  & wsl.exe @DistroArgs -u root -- @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "$FailureMessage (exit $LASTEXITCODE)"
  }
}

<#
.SYNOPSIS
  Traduce C:\ruta\del\proyecto a /mnt/c/ruta/del/proyecto.
.DESCRIPTION
  Lo hace aca en vez de delegar en `wslpath` a proposito: PowerShell 5.1 se
  come las barras invertidas al armar la linea de comandos de un ejecutable
  nativo, y `wslpath` termina recibiendo "C:NicolasProyectos". Para una ruta con
  letra de unidad la traduccion es trivial y deterministica, asi que no vale la
  pena el viaje de ida y vuelta.
#>
function ConvertTo-WslPath {
  param([Parameter(Mandatory)][string]$WindowsPath)

  $full = [System.IO.Path]::GetFullPath($WindowsPath)
  if ($full -notmatch '^[A-Za-z]:\\') {
    throw "El proyecto tiene que estar en una unidad local para compilarlo en WSL; '$full' no lo esta."
  }

  $drive = $full.Substring(0, 1).ToLowerInvariant()
  $rest = $full.Substring(2).Replace('\', '/').TrimEnd('/')
  return "/mnt/$drive$rest"
}

$TotalSteps = if ($LinuxOnly) { 3 } else { 4 }

# --- 1. Verificar que WSL responde ------------------------------------------
Write-Host "==> 1/$TotalSteps  Verificando WSL" -ForegroundColor Cyan
if (-not (Get-Command wsl.exe -ErrorAction SilentlyContinue)) {
  throw "No se encontro wsl.exe. Instala WSL con 'wsl --install' y volve a intentar."
}

# No parseamos la salida de `wsl --list`: viene en UTF-16 y es un dolor de
# cabeza. Que un comando trivial corra adentro es una prueba mas directa.
& wsl.exe @DistroArgs -u root -- true
if ($LASTEXITCODE -ne 0) {
  $which = if ($Distro) { "la distribucion '$Distro'" } else { 'la distribucion predeterminada de WSL' }
  throw "No se pudo ejecutar un comando en $which. Revisa 'wsl --list --verbose'."
}

# Se lee con grep y se parsea aca para no meter comillas ni `$` en la linea de
# comandos, que es donde PowerShell 5.1 y wsl.exe se pisan.
$PrettyName = & wsl.exe @DistroArgs -u root -- grep -m1 '^PRETTY_NAME=' /etc/os-release
if ($PrettyName) {
  Write-Host ('     ' + ($PrettyName -replace '^PRETTY_NAME=', '').Trim('"', ' ', "`r"))
}

# Rutas del proyecto y de la salida, vistas desde WSL.
$SourceWsl = ConvertTo-WslPath $ProjectRoot
$OutputWsl = "$SourceWsl/dist-linux"

# --- 2. Provisionar la toolchain --------------------------------------------
Write-Host "==> 2/$TotalSteps  Preparando la toolchain dentro de WSL" -ForegroundColor Cyan
Invoke-Wsl -Arguments @('bash', "$SourceWsl/scripts/linux-build/provision-wsl.sh") `
  -FailureMessage 'No se pudo preparar la toolchain dentro de WSL.'

# --- 3. Compilar el paquete Linux -------------------------------------------
Write-Host "==> 3/$TotalSteps  Compilando el paquete Linux" -ForegroundColor Cyan
if (Test-Path $LocalOut) { Remove-Item -Recurse -Force $LocalOut }
New-Item -ItemType Directory -Path $LocalOut | Out-Null

Invoke-Wsl -Arguments @('bash', "$SourceWsl/scripts/linux-build/build-in-wsl.sh", $SourceWsl, $OutputWsl) `
  -FailureMessage 'El build de Linux fallo.'

# --- 4. El .exe de Windows, para tener ambos artefactos juntos ---------------
if (-not $LinuxOnly) {
  if (-not $SkipWindowsBuild) {
    Write-Host "==> 4/$TotalSteps  Compilando el .exe de Windows" -ForegroundColor Cyan
    & pnpm tauri build --no-bundle
    if ($LASTEXITCODE -ne 0) { throw 'El build de Windows fallo.' }
  } elseif (-not (Test-Path $Exe)) {
    throw "No existe $Exe y se pidio -SkipWindowsBuild."
  } else {
    Write-Host "==> 4/$TotalSteps  Se omite el build de Windows (se usa el existente)" -ForegroundColor Cyan
  }

  Copy-Item $Exe (Join-Path $LocalOut 'sound-deck.exe')
}

Write-Host ''
Write-Host "OK. Artefactos en: $LocalOut" -ForegroundColor Green
Get-ChildItem $LocalOut | ForEach-Object {
  '  {0,-40} {1,8:N1} MB' -f $_.Name, ($_.Length / 1MB)
}
