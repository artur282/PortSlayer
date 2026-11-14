<p align="center">
  <img src="assets/banner.svg" alt="PortSlayer Banner" width="800"/>
</p>

<p align="center">
  <a href="https://github.com/artur282/portslayer/releases"><img src="https://img.shields.io/github/v/release/artur282/portslayer?style=for-the-badge&color=00d2ff&labelColor=1a1a2e" alt="Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-00d2ff?style=for-the-badge&labelColor=1a1a2e" alt="License"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white&color=00d2ff&labelColor=1a1a2e" alt="Rust"></a>
  <a href="https://github.com/artur282/portslayer/stargazers"><img src="https://img.shields.io/github/stars/artur282/portslayer?style=for-the-badge&color=00d2ff&labelColor=1a1a2e" alt="Stars"></a>
  <a href="https://github.com/artur282/portslayer/issues"><img src="https://img.shields.io/github/issues/artur282/portslayer?style=for-the-badge&color=00d2ff&labelColor=1a1a2e" alt="Issues"></a>
</p>

<p align="center">
  <b>🔥 Visualiza y elimina puertos abiertos directamente desde tu barra de tareas en Linux</b>
</p>

<p align="center">
  <a href="#-instalación">Instalación</a> •
  <a href="#-uso">Uso</a> •
  <a href="#-características">Características</a> •
  <a href="#-capturas">Capturas</a> •
  <a href="#-contribuir">Contribuir</a>
</p>

---

## 🤔 ¿El Problema?

¿Cuántas veces te ha pasado esto?

```
Error: Port 3000 is already in use
Error: Address already in use (os error 98)
```

Y luego tienes que abrir la terminal, buscar el PID, ejecutar `kill`... **demasiados pasos para algo tan simple.**

## ⚔️ La Solución: PortSlayer

**PortSlayer** vive en tu **barra de tareas**. Un clic derecho y ves todos los puertos abiertos. Un clic más y el puerto está libre. Así de fácil.

> 💡 Hecho en **Rust** para máximo rendimiento: usa ~2MB de RAM y 0% CPU en reposo.

---

## ✨ Características

| Característica | Descripción |
|:---:|:---|
| 🖥️ **System Tray** | Vive en tu barra de tareas, siempre accesible |
| 🔍 **Escaneo en tiempo real** | Detecta puertos TCP/UDP abiertos automáticamente |
| ⚡ **Kill instantáneo** | Cierra cualquier puerto con un solo clic |
| 💣 **Kill All** | Cierra todos los puertos abiertos de una vez |
| 🔄 **Auto-actualización** | Se actualiza cada 10 segundos automáticamente |
| 🚀 **Autostart** | Se inicia con tu sistema automáticamente |
| 🔒 **Permisos inteligentes** | Solicita permisos elevados solo cuando es necesario |
| 🪶 **Ultra ligero** | ~2MB RAM, binario estático de ~3MB |
| 🐧 **100% Linux** | Compatible con ZorinOS, Ubuntu, Mint, Fedora y más |

---

## 📸 Capturas

### Menú del System Tray
```
┌─────────────────────────────────────────────┐
│  🔄 Actualizar                              │
│  ─────────────────────────────────────────── │
│  ⚔️ Cerrar Todos (4 puertos)                │
│  ─────────────────────────────────────────── │
│  🔴 Kill: Puerto 3000: node (PID: 12345)    │
│  🔴 Kill: Puerto 5432: postgres (PID: 987)  │
│  🔴 Kill: Puerto 8080: java (PID: 5678)     │
│  🔴 Kill: Puerto 9090: python (PID: 4321)   │
│  ─────────────────────────────────────────── │
│  ❌ Salir                                    │
└─────────────────────────────────────────────┘
```

---

## 📦 Instalación

### Método rápido (recomendado)

```bash
# Clonar el repositorio
git clone https://github.com/artur282/PortSlayer.git
cd PortSlayer

# Instalar (compila, instala en ~/.local y configura autostart)
chmod +x scripts/install.sh
./scripts/install.sh
```

El instalador es user-local y **no usa `sudo`** para instalar el binario. Deja:

- `~/.local/bin/portslayer`
- `~/.local/share/applications/portslayer.desktop`
- `~/.config/autostart/portslayer.desktop`

Podés elegir otro prefijo y omitir la compilación si el binario ya existe:

```bash
PREFIX="$HOME/.local" ./scripts/install.sh
./scripts/install.sh --no-build
```

Para ver los PIDs de todos los procesos, el instalador puede crear una regla
opcional de `sudoers` que permite `sudo -n ss`. Es opt-in y requiere `sudo`:

```bash
./scripts/install.sh --enable-ss-sudo
```

Sin esa regla, PortSlayer usa `ss` sin privilegios y `pkexec` para cerrar
procesos que no te pertenecen.

### Compilar manualmente

```bash
# Instalar dependencias del sistema
sudo apt install -y libdbus-1-dev pkg-config   # Debian/Ubuntu/Zorin
sudo dnf install -y dbus-devel pkg-config       # Fedora
sudo pacman -S dbus pkg-config                  # Arch

# Compilar
cargo build --release --locked

# El binario está en target/release/portslayer
./target/release/portslayer &
```

### Desinstalar

```bash
chmod +x scripts/uninstall.sh
./scripts/uninstall.sh
```

También podés usar `./scripts/install.sh --uninstall`. El desinstalador elimina
el binario, el desktop entry, el autostart y la regla de `sudoers` si existe.

---

## 🚀 Uso

### Iniciar manualmente
```bash
portslayer &
```

### Iniciar con logs visibles
```bash
RUST_LOG=debug portslayer
```

### Comportamiento
1. **Inicia con el sistema** automáticamente (después de instalar)
2. **Clic derecho** en el ícono 🖥️ de la barra de tareas
3. **Ver** todos los puertos TCP/UDP abiertos con sus procesos
4. **Clic** en cualquier puerto para cerrarlo instantáneamente
5. **"Cerrar Todos"** para liberar todos los puertos de una vez

---

## 🏗️ Arquitectura

```
portslayer/
├── src/
│   ├── main.rs            # Punto de entrada y configuración de logging
│   ├── port_scanner.rs    # Escaneo de puertos y gestión de procesos
│   └── tray.rs            # System tray con menú dinámico
├── scripts/
│   ├── install.sh         # Instalador automático
│   └── uninstall.sh       # Desinstalador limpio
├── assets/
│   └── banner.svg         # Banner del README
├── Cargo.toml             # Configuración del proyecto Rust
├── LICENSE                # Licencia MIT
└── README.md              # Este archivo
```

### Dependencias Rust

| Crate | Uso |
|:---|:---|
| [`ksni`](https://crates.io/crates/ksni) | System tray con protocolo StatusNotifierItem |
| [`log`](https://crates.io/crates/log) | Framework de logging |
| [`env_logger`](https://crates.io/crates/env_logger) | Backend de logging configurable |

### Herramientas del sistema

| Herramienta | Uso |
|:---|:---|
| `ss` | Escaneo de sockets/puertos de red |
| `kill` | Terminación de procesos |
| `pkexec` | Escalamiento de privilegios con GUI |

---

## 🐧 Distros Compatibles

| Distribución | Estado |
|:---|:---:|
| ZorinOS 16/17 | ✅ Probado |
| Ubuntu 20.04+ | ✅ Compatible |
| Linux Mint 20+ | ✅ Compatible |
| Fedora 36+ | ✅ Compatible |
| Arch Linux | ✅ Compatible |
| Pop!\_OS | ✅ Compatible |
| Debian 11+ | ✅ Compatible |
| Elementary OS | ✅ Compatible |

> ⚠️ Requiere un entorno de escritorio con soporte para **StatusNotifierItem** o **AppIndicator** (GNOME con extensión, KDE, XFCE, Budgie, etc.)

---

## 🔧 Configuración

### Variables de entorno

| Variable | Descripción | Default |
|:---|:---|:---|
| `RUST_LOG` | Nivel de logging (`error`, `warn`, `info`, `debug`, `trace`) | `info` |

### Autostart

El instalador crea automáticamente un archivo `.desktop` en:
```
~/.config/autostart/portslayer.desktop
```

Para **desactivar** el autostart sin desinstalar:
```bash
rm ~/.config/autostart/portslayer.desktop
```

---

## 🤝 Contribuir

¡Las contribuciones son bienvenidas! 🎉

1. **Fork** el repositorio
2. Crea tu **branch** (`git checkout -b feature/nueva-funcionalidad`)
3. **Commit** tus cambios (`git commit -m 'feat: agregar funcionalidad'`)
4. **Push** al branch (`git push origin feature/nueva-funcionalidad`)
5. Abre un **Pull Request**

### Ideas para contribuir
- [ ] 🎨 Ícono personalizado SVG para el system tray
- [ ] 📊 Notificaciones cuando un nuevo puerto se abre
- [ ] 🔍 Filtrar puertos por protocolo (TCP/UDP)
- [ ] 📋 Copiar información del puerto al portapapeles
- [ ] 🌐 Interfaz web opcional para monitoreo remoto
- [ ] 📦 Paquetes `.deb`, `.rpm` y AUR
- [ ] 🎯 Whitelist/Blacklist de puertos

---

## ⭐ ¿Te gusta PortSlayer?

Si este proyecto te es útil, **dale una estrella** ⭐ en GitHub. ¡Ayuda a que más personas lo descubran!

<p align="center">
  <a href="https://github.com/artur282/portslayer">
    <img src="https://img.shields.io/github/stars/artur282/portslayer?style=social" alt="GitHub Stars">
  </a>
</p>

---

## 📝 Licencia

Este proyecto está bajo la licencia [MIT](LICENSE). Puedes usarlo, modificarlo y distribuirlo libremente.

---

<p align="center">
  Hecho con ❤️ y 🦀 Rust
</p>
