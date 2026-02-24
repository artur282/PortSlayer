/// # PortSlayer ⚔️
///
/// Aplicación de bandeja del sistema (system tray) para Linux
/// que permite visualizar y forzar el cierre de puertos de red en uso.
///
/// ## Características
/// - Ícono en la bandeja del sistema con menú contextual
/// - Lista dinámica de puertos TCP/UDP abiertos
/// - Cierre individual o masivo de puertos
/// - Actualización automática cada 10 segundos
/// - Soporte para solicitar permisos elevados vía pkexec
///
/// ## Uso
/// Ejecutar el binario para que aparezca en la bandeja del sistema.
/// Clic derecho sobre el ícono para ver el menú con los puertos.

mod port_scanner;
mod tray;

/// Punto de entrada principal de PortSlayer.
///
/// Inicializa el sistema de logging y lanza el system tray.
/// La aplicación se ejecuta indefinidamente hasta que el usuario
/// seleccione "Salir" del menú contextual.
fn main() {
    // Inicializar logging (nivel INFO por defecto, configurable con RUST_LOG)
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .format_timestamp_secs()
    .init();

    log::info!("⚔️  PortSlayer v{} iniciando...", env!("CARGO_PKG_VERSION"));
    log::info!("Sistema de monitoreo de puertos para Linux");

    // Lanzar el system tray (bloquea el hilo principal)
    tray::run_tray();
}
