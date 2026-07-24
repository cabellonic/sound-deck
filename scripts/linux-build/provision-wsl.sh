#!/usr/bin/env bash
#
# Instala la toolchain necesaria para compilar el paquete Linux dentro de WSL.
#
# Corre como root y es idempotente: la primera vez tarda varios minutos y las
# siguientes termina en menos de un segundo. Se ejecuta como root a proposito,
# igual que corria el build dentro del contenedor Docker que este flujo
# reemplaza: la distro de WSL es un entorno de compilacion descartable, no un
# sistema multiusuario, y asi apt no pide contrasena ni hay permisos cruzados
# entre el cache de cargo y el usuario que dispara el build.
set -euo pipefail

# Subir este numero fuerza a reprovisionar aunque el sello ya exista.
RECIPE_VERSION=1
STAMP=/root/.sound-deck-toolchain

log() { printf '\033[36m==>\033[0m %s\n' "$1"; }

# Cargo se instala fuera del PATH por defecto, asi que hay que cargarlo antes de
# preguntar si ya esta. Sin esto, la comprobacion de abajo nunca encuentra
# `cargo` y cada corrida vuelve a bajar y ejecutar el instalador de rustup.
if [[ -f /root/.cargo/env ]]; then
  # shellcheck disable=SC1091
  source /root/.cargo/env
fi

toolchain_ready() {
  [[ -f "$STAMP" && "$(cat "$STAMP")" == "$RECIPE_VERSION" ]] || return 1
  # El sello solo vale si las herramientas siguen estando: alcanza con que
  # alguien haya limpiado la distro para que mienta.
  for tool in cargo rustc node pnpm rsync; do
    command -v "$tool" >/dev/null 2>&1 || return 1
  done
  pkg-config --exists webkit2gtk-4.1 || return 1
}

if toolchain_ready; then
  log "Toolchain ya instalada (receta v$RECIPE_VERSION)"
  exit 0
fi

log "Instalando dependencias de sistema (la primera vez tarda)"
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends \
  build-essential \
  ca-certificates \
  curl \
  file \
  pkg-config \
  rsync \
  xz-utils \
  libssl-dev \
  libglib2.0-dev \
  libgtk-3-dev \
  libwebkit2gtk-4.1-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libasound2-dev \
  dpkg-dev

# Tauri 2 se compila contra webkit2gtk 4.1. Si la distro dejo de traerlo, es
# mejor decirlo aca que fallar mil lineas despues dentro de cargo.
if ! pkg-config --exists webkit2gtk-4.1; then
  echo "ERROR: esta distro no provee webkit2gtk-4.1, que es lo que necesita Tauri 2." >&2
  echo "       Proba con una version de Ubuntu que todavia lo empaquete." >&2
  exit 1
fi

if ! command -v rustc >/dev/null 2>&1; then
  log "Instalando Rust (perfil minimo)"
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable --no-modify-path
  # shellcheck disable=SC1091
  source /root/.cargo/env
fi

if ! command -v node >/dev/null 2>&1; then
  # Node desde el tarball oficial y no desde apt ni NodeSource: no depende de
  # que exista un repo para esta version de Ubuntu, que es justo lo que se
  # rompe en una distro recien salida.
  log "Buscando el ultimo Node 22 LTS"
  NODE_TARBALL=$(curl -fsSL https://nodejs.org/dist/latest-v22.x/ \
    | grep -o 'node-v22\.[0-9.]*-linux-x64\.tar\.xz' | head -1)
  if [[ -z "$NODE_TARBALL" ]]; then
    echo "ERROR: no se pudo determinar la version de Node a instalar." >&2
    exit 1
  fi

  log "Instalando ${NODE_TARBALL%-linux-x64.tar.xz}"
  curl -fsSL "https://nodejs.org/dist/latest-v22.x/$NODE_TARBALL" -o /tmp/node.tar.xz
  tar -xJf /tmp/node.tar.xz -C /usr/local --strip-components=1
  rm -f /tmp/node.tar.xz
fi

if ! command -v pnpm >/dev/null 2>&1; then
  log "Activando pnpm con corepack"
  corepack enable
  # Sin version: corepack respeta el campo `packageManager` del package.json.
  corepack prepare --activate
fi

echo "$RECIPE_VERSION" >"$STAMP"

log "Toolchain lista"
printf '    rust  %s\n' "$(rustc --version)"
printf '    node  %s\n' "$(node --version)"
printf '    pnpm  %s\n' "$(pnpm --version)"
