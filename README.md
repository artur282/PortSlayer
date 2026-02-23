# Puertos Guard 🛡️

Una aplicación moderna y eficiente para Linux (ZorinOS/Ubuntu) construida con **Rust** y **Tauri** que permite visualizar y cerrar procesos que están utilizando puertos de red.

## ✨ Características
- **Visualización en tiempo real**: Lista todos los puertos TCP/UDP en estado LISTEN.
- **Cierre Forzado**: Mata procesos específicos desde la interfaz.
- **Kill All**: Cierra todos los procesos que ocupan puertos con un solo clic.
- **Interfaz Premium**: Diseño oscuro con efectos de glassmorphism y animaciones suaves.

## 🚀 Requisitos previos
Para compilar y ejecutar esta aplicación, necesitas tener instalado Rust y las dependencias de desarrollo de Tauri en ZorinOS:

```bash
# Instalar Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Instalar dependencias del sistema
sudo apt update
sudo apt install -y libgtk-3-dev libwebkit2gtk-4.0-dev libappindicator3-dev librsvg2-dev patchelf
```

## 🛠️ Ejecución

```bash
# Instalar el CLI de Tauri
cargo install tauri-cli

# Ejecutar en modo desarrollo
cargo tauri dev
```

## 📦 Construcción (Release)

```bash
cargo tauri build
```
El instalador `.deb` se generará en `target/release/bundle/deb/`.

## 📸 Captura de Pantalla
![App Screenshot](ui/index.html) *(La interfaz utiliza Glassmorphism y fuentes Inter)*

---
Desarrollado con ❤️ para ZorinOS.
