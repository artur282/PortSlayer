#!/bin/bash
# =====================================================
# PortSlayer ⚔️ - Script de Desinstalación
# =====================================================
# Elimina PortSlayer del sistema completamente
# =====================================================

set -e

# Colores para la salida
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

INSTALL_DIR="/usr/local/bin"
AUTOSTART_DIR="$HOME/.config/autostart"
BINARY_NAME="portslayer"
DESKTOP_FILE="portslayer.desktop"

echo -e "${CYAN}"
echo "  ⚔️  PortSlayer - Desinstalador"
echo "  ================================"
echo -e "${NC}"

# ─── Detener proceso si está corriendo ─────────────────
echo -e "${YELLOW}[1/4]${NC} Deteniendo PortSlayer si está en ejecución..."
if pgrep -x "$BINARY_NAME" > /dev/null 2>&1; then
    pkill -x "$BINARY_NAME" && echo -e "${GREEN}✓ Proceso detenido${NC}"
else
    echo -e "${GREEN}✓ No está en ejecución${NC}"
fi

# ─── Eliminar binario ─────────────────────────────────
echo -e "${YELLOW}[2/4]${NC} Eliminando binario..."
if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
    sudo rm -f "$INSTALL_DIR/$BINARY_NAME"
    echo -e "${GREEN}✓ Binario eliminado${NC}"
else
    echo -e "${GREEN}✓ Binario no encontrado (ya eliminado)${NC}"
fi

# ─── Eliminar autostart ───────────────────────────────
echo -e "${YELLOW}[3/4]${NC} Eliminando autostart..."
if [ -f "$AUTOSTART_DIR/$DESKTOP_FILE" ]; then
    rm -f "$AUTOSTART_DIR/$DESKTOP_FILE"
    echo -e "${GREEN}✓ Autostart eliminado${NC}"
else
    echo -e "${GREEN}✓ Autostart no encontrado (ya eliminado)${NC}"
fi

# ─── Eliminar sudoers ─────────────────────────────────
echo -e "${YELLOW}[4/4]${NC} Eliminando configuración de sudoers..."
SUDOERS_FILE="/etc/sudoers.d/portslayer"
if [ -f "$SUDOERS_FILE" ]; then
    sudo rm -f "$SUDOERS_FILE"
    echo -e "${GREEN}✓ Sudoers eliminado${NC}"
else
    echo -e "${GREEN}✓ Sudoers no encontrado (ya eliminado)${NC}"
fi

echo ""
echo -e "${CYAN}  ⚔️  PortSlayer desinstalado completamente.${NC}"
echo ""
