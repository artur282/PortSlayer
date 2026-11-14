#!/usr/bin/env bash
# Instalador user-local de PortSlayer.
# No usa sudo salvo que se pida --enable-ss-sudo o al desinstalar un archivo
# de sudoers creado por una version anterior.

set -euo pipefail

APP_NAME="PortSlayer"
BINARY_NAME="portslayer"
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
PREFIX="${PREFIX:-$HOME/.local}"
BINDIR="$PREFIX/bin"
APPDIR="$PREFIX/share/applications"
DESKTOP_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/autostart"
DESKTOP_NAME="portslayer.desktop"
SUDOERS_FILE="/etc/sudoers.d/portslayer"

usage() {
    cat <<EOF
Instala $APP_NAME en un prefijo de usuario.

Uso: scripts/install.sh [--build|--no-build] [--uninstall] [--enable-ss-sudo]

Opciones:
  --build             Compila el binario release antes de instalar
  --no-build          Falla si el binario no existe en vez de compilar
  --uninstall         Elimina los archivos instalados
  --enable-ss-sudo    Permite ejecutar 'sudo -n ss' sin contrasena para ver
                      los PIDs de todos los procesos. Es opcional y requiere
                      sudo.
  -h, --help          Muestra esta ayuda

Entorno:
  PREFIX              Prefijo de instalacion (default: $HOME/.local)
EOF
}

refresh_caches() {
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APPDIR" >/dev/null 2>&1 || true
    fi
}

check_build_dependencies() {
    if ! command -v pkg-config >/dev/null 2>&1 ||
       ! pkg-config --exists dbus-1 2>/dev/null; then
        echo "error: falta libdbus-1-dev (pkg-config no encuentra dbus-1)." >&2
        echo "Instalala con uno de estos comandos:" >&2
        echo "  Debian/Ubuntu: sudo apt install -y libdbus-1-dev pkg-config" >&2
        echo "  Fedora:        sudo dnf install -y dbus-devel pkg-config" >&2
        echo "  Arch:          sudo pacman -S --noconfirm dbus pkg-config" >&2
        exit 1
    fi
}

BUILD_MODE="auto"
UNINSTALL=0
ENABLE_SS_SUDO=0

for arg in "$@"; do
    case "$arg" in
        --build) BUILD_MODE="always" ;;
        --no-build) BUILD_MODE="never" ;;
        --uninstall) UNINSTALL=1 ;;
        --enable-ss-sudo) ENABLE_SS_SUDO=1 ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "error: opcion desconocida: $arg" >&2
            usage >&2
            exit 2
            ;;
    esac
done

if [ "$UNINSTALL" -eq 1 ]; then
    if pgrep -x "$BINARY_NAME" >/dev/null 2>&1; then
        pkill -x "$BINARY_NAME" || true
    fi
    rm -f \
        "$BINDIR/$BINARY_NAME" \
        "$APPDIR/$DESKTOP_NAME" \
        "$DESKTOP_DIR/$DESKTOP_NAME"
    if [ -f "$SUDOERS_FILE" ]; then
        echo "Eliminando $SUDOERS_FILE (requiere sudo)..."
        sudo rm -f "$SUDOERS_FILE"
    fi
    refresh_caches
    echo "PortSlayer fue eliminado de $PREFIX."
    exit 0
fi

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
fi

case "$BUILD_MODE" in
    always)
        check_build_dependencies
        (cd "$ROOT" && cargo build --release --locked)
        ;;
    auto)
        if [ ! -x "$ROOT/target/release/$BINARY_NAME" ] &&
           [ ! -x "$ROOT/$BINARY_NAME" ]; then
            check_build_dependencies
            (cd "$ROOT" && cargo build --release --locked)
        fi
        ;;
    never)
        ;;
esac

if [ -x "$ROOT/target/release/$BINARY_NAME" ]; then
    BINARY="$ROOT/target/release/$BINARY_NAME"
elif [ -x "$ROOT/$BINARY_NAME" ]; then
    BINARY="$ROOT/$BINARY_NAME"
else
    echo "error: no se encontro el binario release." >&2
    echo "Compilalo con: cargo build --release --locked" >&2
    exit 1
fi

DESKTOP_SOURCE="$ROOT/packaging/$DESKTOP_NAME"
if [ ! -f "$DESKTOP_SOURCE" ]; then
    echo "error: falta $DESKTOP_SOURCE" >&2
    exit 1
fi

EXEC_PATH="$BINDIR/$BINARY_NAME"
case "$EXEC_PATH" in
    *[[:space:]]*)
        echo "error: PREFIX no puede contener espacios." >&2
        exit 1
        ;;
esac

mkdir -p "$BINDIR" "$APPDIR" "$DESKTOP_DIR"

install -m755 "$BINARY" "$BINDIR/$BINARY_NAME"

TMP_DESKTOP=$(mktemp)
trap 'rm -f "$TMP_DESKTOP"' EXIT
EXEC_LINE="Exec=$EXEC_PATH" awk '
    /^Exec=/ { print ENVIRON["EXEC_LINE"]; next }
    { print }
' "$DESKTOP_SOURCE" > "$TMP_DESKTOP"
install -m644 "$TMP_DESKTOP" "$APPDIR/$DESKTOP_NAME"
rm -f "$TMP_DESKTOP"
trap - EXIT

cat > "$DESKTOP_DIR/$DESKTOP_NAME" <<EOF
[Desktop Entry]
Type=Application
Name=PortSlayer
Comment=Monitor and kill open ports from system tray
Exec=$EXEC_PATH
Icon=network-server
Terminal=false
Categories=System;
StartupNotify=false
X-GNOME-Autostart-enabled=true
X-GNOME-Autostart-Delay=5
EOF

if [ "$ENABLE_SS_SUDO" -eq 1 ]; then
    echo "Configurando sudo -n para /usr/sbin/ss (requiere sudo)..."
    SUDOERS_TMP=$(mktemp)
    trap 'rm -f "$SUDOERS_TMP"' EXIT
    printf '%s ALL=(ALL) NOPASSWD: /usr/sbin/ss\n' "${USER:-$(id -un)}" \
        > "$SUDOERS_TMP"
    if command -v visudo >/dev/null 2>&1; then
        visudo -cf "$SUDOERS_TMP" >/dev/null
    fi
    sudo install -m440 "$SUDOERS_TMP" "$SUDOERS_FILE"
    rm -f "$SUDOERS_TMP"
    trap - EXIT
fi

refresh_caches

echo "PortSlayer instalado:"
echo "  binario:  $BINDIR/$BINARY_NAME"
echo "  desktop:  $APPDIR/$DESKTOP_NAME"
echo "  autostart: $DESKTOP_DIR/$DESKTOP_NAME"
if [ "$ENABLE_SS_SUDO" -eq 1 ]; then
    echo "  sudoers:  $SUDOERS_FILE"
fi
echo

case ":$PATH:" in
    *":$BINDIR:"*)
        echo "Ejecuta: $BINARY_NAME &"
        ;;
    *)
        echo "nota: $BINDIR no esta en tu PATH."
        echo "Agregalo con:"
        echo "  export PATH=\"$BINDIR:\$PATH\""
        echo "Luego ejecuta: $BINARY_NAME &"
        ;;
esac
