/// Módulo del system tray (bandeja del sistema).
/// Implementa el ícono y menú contextual de PortSlayer
/// usando el protocolo StatusNotifierItem/AppIndicator de Linux.
use ksni::{self, menu::StandardItem, Tray};
use std::process;
use std::sync::{Arc, Mutex};

use crate::port_scanner;

/// Estado compartido del tray que mantiene la lista de puertos
/// actualizada entre el hilo del tray y el hilo de actualización.
#[derive(Debug)]
pub struct PortSlayerTray {
    /// Lista de puertos abiertos detectados actualmente
    ports: Arc<Mutex<Vec<port_scanner::PortInfo>>>,
}

impl PortSlayerTray {
    /// Crea una nueva instancia del tray con escaneo inicial de puertos.
    pub fn new() -> Self {
        let ports = port_scanner::scan_open_ports();
        Self {
            ports: Arc::new(Mutex::new(ports)),
        }
    }

    /// Obtiene una referencia al Arc de puertos para compartir con otros hilos.
    pub fn ports_handle(&self) -> Arc<Mutex<Vec<port_scanner::PortInfo>>> {
        Arc::clone(&self.ports)
    }
}

impl Tray for PortSlayerTray {
    /// Nombre del ícono a mostrar en el system tray.
    /// Usa "network-server" que está disponible en la mayoría de temas de íconos.
    fn icon_name(&self) -> String {
        "network-server".into()
    }

    /// Tooltip que aparece al pasar el ratón sobre el ícono.
    fn title(&self) -> String {
        "PortSlayer ⚔️".into()
    }

    /// ID único de la aplicación para el protocolo StatusNotifierItem.
    fn id(&self) -> String {
        "portslayer".into()
    }

    /// Construye el menú contextual que se muestra al hacer clic derecho.
    ///
    /// El menú se reconstruye dinámicamente cada vez que se abre,
    /// mostrando los puertos actuales con opción de cerrarlos.
    ///
    /// Estructura del menú:
    /// - 🔄 Actualizar
    /// - ──────────
    /// - ⚔️ Cerrar Todos los Puertos
    /// - ──────────
    /// - 🔴 Puerto XXXX: nombre_proceso (PID: YYYY)
    /// - 🔴 Puerto XXXX: nombre_proceso (PID: YYYY)
    /// - ...
    /// - ──────────
    /// - ❌ Salir
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let mut items: Vec<ksni::MenuItem<Self>> = Vec::new();

        // Botón de actualizar la lista de puertos
        items.push(
            StandardItem {
                label: "🔄 Actualizar".into(),
                activate: Box::new(|tray: &mut Self| {
                    log::info!("Actualizando lista de puertos...");
                    let new_ports = port_scanner::scan_open_ports();
                    if let Ok(mut ports) = tray.ports.lock() {
                        *ports = new_ports;
                    }
                }),
                ..Default::default()
            }
            .into(),
        );

        // Separador visual
        items.push(ksni::MenuItem::Separator);

        // Obtener snapshot actual de los puertos
        let current_ports = match self.ports.lock() {
            Ok(ports) => ports.clone(),
            Err(_) => Vec::new(),
        };

        if current_ports.is_empty() {
            // Mensaje cuando no hay puertos abiertos
            items.push(
                StandardItem {
                    label: "✅ No hay puertos abiertos".into(),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
            );
        } else {
            // Botón para cerrar TODOS los puertos
            items.push(
                StandardItem {
                    label: format!(
                        "⚔️ Cerrar Todos ({} puertos)",
                        current_ports.len()
                    ),
                    activate: Box::new(|tray: &mut Self| {
                        log::info!("Cerrando todos los puertos...");
                        match port_scanner::kill_all_port_processes() {
                            Ok(count) => {
                                log::info!("{} procesos terminados", count);
                            }
                            Err(e) => {
                                log::error!("Error al cerrar puertos: {}", e);
                            }
                        }
                        // Actualizar la lista después de cerrar
                        let new_ports = port_scanner::scan_open_ports();
                        if let Ok(mut ports) = tray.ports.lock() {
                            *ports = new_ports;
                        }
                    }),
                    ..Default::default()
                }
                .into(),
            );

            items.push(ksni::MenuItem::Separator);

            // Crear un item por cada puerto abierto con opción de cerrarlo
            for port_info in &current_ports {
                let pid = port_info.pid;
                let port_num = port_info.port;
                let label = format!(
                    "🔴 Kill: Puerto {}: {} (PID: {})",
                    port_info.port, port_info.process_name, port_info.pid
                );

                items.push(
                    StandardItem {
                        label,
                        activate: Box::new(move |tray: &mut Self| {
                            log::info!(
                                "Cerrando puerto {} (PID: {})",
                                port_num,
                                pid
                            );
                            match port_scanner::kill_process(pid) {
                                Ok(()) => {
                                    log::info!(
                                        "Puerto {} cerrado exitosamente",
                                        port_num
                                    );
                                }
                                Err(e) => {
                                    log::error!(
                                        "Error cerrando puerto {}: {}",
                                        port_num,
                                        e
                                    );
                                }
                            }
                            // Actualizar la lista después de cerrar
                            let new_ports = port_scanner::scan_open_ports();
                            if let Ok(mut ports) = tray.ports.lock() {
                                *ports = new_ports;
                            }
                        }),
                        ..Default::default()
                    }
                    .into(),
                );
            }
        }

        // Separador final
        items.push(ksni::MenuItem::Separator);

        // Botón de salir
        items.push(
            StandardItem {
                label: "❌ Salir".into(),
                activate: Box::new(|_: &mut Self| {
                    log::info!("PortSlayer cerrándose...");
                    process::exit(0);
                }),
                ..Default::default()
            }
            .into(),
        );

        items
    }
}

/// Inicia el system tray y ejecuta el loop principal.
///
/// Crea el ícono en la bandeja del sistema y lanza un hilo
/// de actualización automática que refresca la lista de puertos
/// cada 10 segundos.
///
/// # Panics
/// Si no se puede crear el servicio del system tray (ej: no hay
/// bandeja del sistema disponible en el entorno de escritorio).
pub fn run_tray() {
    log::info!("Iniciando PortSlayer system tray...");

    let tray = PortSlayerTray::new();
    let ports_handle = tray.ports_handle();

    // Crear el servicio del system tray
    let service = ksni::TrayService::new(tray);
    let handle = service.handle();

    // Hilo de actualización automática cada 10 segundos
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(10));

            // Escanear puertos actualizados
            let new_ports = port_scanner::scan_open_ports();

            // Actualizar el estado compartido
            if let Ok(mut ports) = ports_handle.lock() {
                *ports = new_ports;
            }

            // Notificar al tray que necesita reconstruir el menú
            handle.update(|_tray: &mut PortSlayerTray| {
                log::debug!("Menú actualizado automáticamente");
            });
        }
    });

    // Ejecutar el servicio (bloquea el hilo principal)
    if let Err(e) = service.run() {
        log::error!("Error ejecutando el servicio de tray: {}", e);
    }
}
