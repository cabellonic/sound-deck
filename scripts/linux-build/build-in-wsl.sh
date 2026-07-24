#!/usr/bin/env bash
#
# Compila el paquete Linux de Sound Deck dentro de WSL.
#
#   build-in-wsl.sh <codigo-fuente> <carpeta-de-salida>
#
# Ambas rutas son rutas de WSL (normalmente bajo /mnt/c). El codigo se copia al
# sistema de archivos nativo de la distro antes de compilar: cargo y pnpm hacen
# decenas de miles de operaciones sobre archivos chicos, y sobre /mnt/c (que
# atraviesa el puente 9p hacia NTFS) eso cuesta varias veces mas que sobre ext4.
set -euo pipefail

SOURCE_DIR=${1:?Falta la ruta del codigo fuente}
OUTPUT_DIR=${2:?Falta la carpeta de salida}
WORK_DIR=${SOUND_DECK_WORK_DIR:-/root/sound-deck-build}

log() { printf '\033[36m==>\033[0m %s\n' "$1"; }

# shellcheck disable=SC1091
source /root/.cargo/env

log "Copiando el codigo a $WORK_DIR"
mkdir -p "$WORK_DIR"
# Las exclusiones cumplen dos funciones: no arrastrar cosas pesadas desde
# Windows, y proteger de --delete lo que vive solo del lado de WSL. node_modules
# y src-tauri/target se quedan entre corridas y son casi todo el tiempo de una
# compilacion desde cero.
rsync -a --delete \
  --exclude='/node_modules/' \
  --exclude='/dist/' \
  --exclude='/dist-linux/' \
  --exclude='/.git/' \
  --exclude='/src-tauri/target/' \
  --exclude='/src-tauri/gen/' \
  "$SOURCE_DIR/" "$WORK_DIR/"

cd "$WORK_DIR"

log "Instalando dependencias del frontend"
pnpm install --frozen-lockfile

log "Compilando el paquete Linux (.deb + binario)"
# Solo .deb: es el artefacto reproducible. El AppImage necesita FUSE, que en
# WSL no esta disponible sin configuracion extra y no aporta nada aca.
pnpm tauri build --bundles deb

log "Copiando los artefactos a la carpeta de salida"
mkdir -p "$OUTPUT_DIR"
cp -v src-tauri/target/release/sound-deck "$OUTPUT_DIR/"
find src-tauri/target/release/bundle -name '*.deb' -exec cp -v {} "$OUTPUT_DIR/" \;

log "Listo"
ls -lh "$OUTPUT_DIR"
