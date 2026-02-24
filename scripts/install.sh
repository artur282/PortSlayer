#!/bin/bash
# =====================================================
# PortSlayer ⚔️ - Script de Instalación
# =====================================================
# Compila, instala y configura el autostart de PortSlayer
# Compatible con: ZorinOS, Ubuntu, Linux Mint, Fedora, Arch
# =====================================================

set -e

# Colores para la salida
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # Sin color

# Rutas de instalación
INSTALL_DIR="/usr/local/bin"
AUTOSTART_DIR="$HOME/.config/autostart"
DESKTOP_FILE="portslayer.desktop"
BINARY_NAME="portslayer"

echo -e "${CYAN}"
echo "  ⚔️  PortSlayer - Instalador"
echo "  ================================"
echo -e "${NC}"

# ─── Verificar dependencias ────────────────────────────
echo -e "${BLUE}[1/5]${NC} Verificando dependencias..."

# Verificar que Rust/Cargo estén instalados
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}⚠ Cargo no encontrado. Instalando Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Verificar dependencia del sistema: libdbus (requerida por ksni)
if ! pkg-config --exists dbus-1 2>/dev/null; then
    echo -e "${YELLOW}⚠ libdbus-1-dev no encontrada. Instalando...${NC}"
    if command -v apt &> /dev/null; then
        sudo apt install -y libdbus-1-dev pkg-config
    elif command -v dnf &> /dev/null; then
        sudo dnf install -y dbus-devel pkg-config
    elif command -v pacman &> /dev/null; then
        sudo pacman -S --noconfirm dbus pkg-config
    fi
fi

echo -e "${GREEN}✓ Dependencias verificadas${NC}"

# ─── Compilar en modo release ──────────────────────────
echo -e "${BLUE}[2/5]${NC} Compilando PortSlayer (modo release)..."
cargo build --release

if [ ! -f "target/release/$BINARY_NAME" ]; then
    echo -e "${RED}✗ Error: No se encontró el binario compilado${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Compilación exitosa${NC}"

# ─── Instalar binario ─────────────────────────────────
echo -e "${BLUE}[3/5]${NC} Instalando binario en $INSTALL_DIR..."
sudo cp "target/release/$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"

echo -e "${GREEN}✓ Binario instalado${NC}"

# ─── Configurar autostart ─────────────────────────────
echo -e "${BLUE}[4/5]${NC} Configurando inicio automático..."
mkdir -p "$AUTOSTART_DIR"

cat > "$AUTOSTART_DIR/$DESKTOP_FILE" << EOF
[Desktop Entry]
Type=Application
Name=PortSlayer
Comment=Monitor and kill open ports from system tray
Exec=$INSTALL_DIR/$BINARY_NAME
Icon=network-server
Terminal=false
Categories=System;Network;Monitor;
StartupNotify=false
X-GNOME-Autostart-enabled=true
X-GNOME-Autostart-Delay=5
EOF

echo -e "${GREEN}✓ Autostart configurado${NC}"

# ─── Configurar sudoers (opcional) ─────────────────────
echo -e "${BLUE}[5/5]${NC} Configurando permisos..."

# Permitir ejecutar ss sin contraseña para ver todos los puertos
SUDOERS_FILE="/etc/sudoers.d/portslayer"
if [ ! -f "$SUDOERS_FILE" ]; then
    echo -e "${YELLOW}→ Configurando acceso a ss sin contraseña...${NC}"
    echo "$USER ALL=(ALL) NOPASSWD: /usr/sbin/ss" | sudo tee "$SUDOERS_FILE" > /dev/null
    sudo chmod 440 "$SUDOERS_FILE"
    echo -e "${GREEN}✓ Permisos configurados${NC}"
else
    echo -e "${GREEN}✓ Permisos ya configurados${NC}"
fi

# ─── Resumen ──────────────────────────────────────────
echo ""
echo -e "${CYAN}  ⚔️  PortSlayer instalado exitosamente!${NC}"
echo -e "  ${GREEN}────────────────────────────────────${NC}"
echo -e "  📍 Binario:   $INSTALL_DIR/$BINARY_NAME"
echo -e "  🔄 Autostart: $AUTOSTART_DIR/$DESKTOP_FILE"
echo -e ""
echo -e "  ${YELLOW}Para iniciar ahora:${NC}"
echo -e "    portslayer &"
echo -e ""
echo -e "  ${YELLOW}La app se iniciará automáticamente con el sistema.${NC}"
echo ""
