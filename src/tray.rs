/// Módulo del system tray (bandeja del sistema).
///
/// Implementa el ícono y menú contextual de PortSlayer usando el
/// protocolo StatusNotifierItem/AppIndicator de Linux.
///
/// ## Características del menú:
/// - Filtro por protocolo (TCP / UDP / Todos)
/// - Paginación configurable (5 o 10 puertos por página)
/// - Navegación entre páginas con indicador visual
/// - Cierre individual y masivo de puertos
/// - Actualización automática cada 10 segundos
use ksni::{self, menu::StandardItem, menu::SubMenu, Tray};
use std::process;
use std::sync::{Arc, Mutex};

use crate::port_scanner::{self, ProtocolFilter};

// ─────────────────────────────────────────────────────────────
// Estado del tray con filtros y paginación
// ─────────────────────────────────────────────────────────────

/// Estado compartido del tray que mantiene la lista de puertos
/// actualizada, junto con la configuración de visualización
/// (filtro de protocolo, página actual, tamaño de página).
#[derive(Debug)]
pub struct PortSlayerTray {
    /// Lista de puertos abiertos detectados actualmente
    ports: Arc<Mutex<Vec<port_scanner::PortInfo>>>,
    /// Filtro de protocolo activo (Todos, TCP, UDP)
    protocol_filter: ProtocolFilter,
    /// Página actual (base 0) de la vista paginada
    current_page: usize,
    /// Cantidad de puertos a mostrar por página
    page_size: usize,
}

/// Tamaño de página por defecto al iniciar la aplicación
const DEFAULT_PAGE_SIZE: usize = 10;

impl PortSlayerTray {
    /// Crea una nueva instancia del tray con escaneo inicial.
    ///
    /// Realiza un escaneo completo de puertos (ss + /proc/net)
    /// y configura la vista con filtro "Todos" y paginación de 10.
    pub fn new() -> Self {
        let ports = port_scanner::scan_open_ports();
        log::info!("Escaneo inicial: {} puertos detectados", ports.len());
        Self {
            ports: Arc::new(Mutex::new(ports)),
            protocol_filter: ProtocolFilter::All,
            current_page: 0,
            page_size: DEFAULT_PAGE_SIZE,
        }
    }

    /// Obtiene una referencia compartida a la lista de puertos.
    ///
    /// Se usa para compartir el estado con el hilo de actualización
    /// automática que refresca los puertos cada 10 segundos.
    pub fn ports_handle(&self) -> Arc<Mutex<Vec<port_scanner::PortInfo>>> {
        Arc::clone(&self.ports)
    }

    /// Actualiza la lista de puertos con un nuevo escaneo.
    ///
    /// Resetea la página actual a 0 ya que la lista puede haber
    /// cambiado y la página anterior podría no existir.
    fn refresh_ports(&mut self) {
        log::info!("Actualizando lista de puertos...");
        let new_ports = port_scanner::scan_open_ports();
        if let Ok(mut ports) = self.ports.lock() {
            *ports = new_ports;
        }
        // Resetear a la primera página tras actualizar
        self.current_page = 0;
    }

    /// Obtiene los puertos filtrados según el filtro de protocolo activo.
    ///
    /// # Returns
    /// Vector con los puertos que coinciden con el filtro actual.
    fn get_filtered_ports(&self) -> Vec<port_scanner::PortInfo> {
        let current_ports = match self.ports.lock() {
            Ok(ports) => ports.clone(),
            Err(_) => Vec::new(),
        };
        port_scanner::filter_ports(&current_ports, self.protocol_filter)
    }
}

// ─────────────────────────────────────────────────────────────
// Implementación del menú contextual del tray
// ─────────────────────────────────────────────────────────────

impl Tray for PortSlayerTray {
    /// Ícono del system tray (usa tema de íconos del sistema).
    fn icon_name(&self) -> String {
        "network-server".into()
    }

    /// Tooltip que aparece al pasar el ratón sobre el ícono.
    fn title(&self) -> String {
        "PortSlayer ⚔️".into()
    }

    /// ID único para el protocolo StatusNotifierItem.
    fn id(&self) -> String {
        "portslayer".into()
    }

    /// Construye el menú contextual dinámico.
    ///
    /// Estructura del menú:
    /// ```text
    /// 🔄 Actualizar
    /// ──────────
    /// 📊 Filtro: [Todos|TCP|UDP] ▸ submenu
    /// 📋 Por página: [5|10] ▸ submenu
    /// ──────────
    /// ⚔️ Cerrar Todos (N puertos)
    /// ──────────
    /// 🔴 TCP 8080 (0.0.0.0) → node [PID 1234]
    /// 🟡 TCP 5434 (0.0.0.0) → desconocido
    /// ...
    /// ──────────
    /// ◀ Anterior | Página X/Y | ▶ Siguiente
    /// ──────────
    /// ❌ Salir
    /// ```
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        let mut items: Vec<ksni::MenuItem<Self>> = vec![
            // ── Botón de actualizar ──
            build_refresh_item(),
            ksni::MenuItem::Separator,
            // ── Filtro de protocolo (submenu) ──
            build_filter_submenu(self.protocol_filter),
            // ── Tamaño de página (submenu) ──
            build_page_size_submenu(self.page_size),
            ksni::MenuItem::Separator,
        ];

        // ── Obtener puertos filtrados y paginados ──
        let filtered_ports = self.get_filtered_ports();
        let total = filtered_ports.len();
        let pages = port_scanner::total_pages(total, self.page_size);

        // Asegurar que la página actual es válida
        let safe_page = self.current_page.min(if pages > 0 { pages - 1 } else { 0 });
        let page_ports = port_scanner::get_page(&filtered_ports, safe_page, self.page_size);

        if total == 0 {
            // Sin puertos abiertos
            items.push(build_empty_message());
        } else {
            // ── Botón cerrar todos ──
            items.push(build_kill_all_item(total));
            items.push(ksni::MenuItem::Separator);

            // ── Encabezado con conteo ──
            items.push(build_count_header(total, self.protocol_filter));

            // ── Lista de puertos de la página actual ──
            for port_info in &page_ports {
                items.push(build_port_item(port_info));
            }
        }

        // ── Navegación de páginas ──
        if pages > 1 {
            items.push(ksni::MenuItem::Separator);
            let nav_items = build_navigation_items(safe_page, pages);
            items.extend(nav_items);
        }

        // ── Botón salir ──
        items.push(ksni::MenuItem::Separator);
        items.push(build_exit_item());

        items
    }
}

// ─────────────────────────────────────────────────────────────
// Constructores de items del menú (mantienen fn menu() limpia)
// ─────────────────────────────────────────────────────────────

/// Construye el item "🔄 Actualizar" del menú.
fn build_refresh_item() -> ksni::MenuItem<PortSlayerTray> {
    StandardItem {
        label: "🔄 Actualizar".into(),
        activate: Box::new(|tray: &mut PortSlayerTray| {
            tray.refresh_ports();
        }),
        ..Default::default()
    }
    .into()
}

/// Construye el submenu de filtro de protocolo.
///
/// Muestra el filtro activo con un indicador ● y permite cambiar
/// entre Todos, TCP y UDP.
///
/// # Arguments
/// * `current_filter` - Filtro actualmente activo
fn build_filter_submenu(current_filter: ProtocolFilter) -> ksni::MenuItem<PortSlayerTray> {
    // Construir las opciones del filtro con indicador visual
    let filters = [
        ProtocolFilter::All,
        ProtocolFilter::Tcp,
        ProtocolFilter::Udp,
    ];

    let submenu_items: Vec<ksni::MenuItem<PortSlayerTray>> = filters
        .iter()
        .map(|&filter| {
            // Indicador visual: ● para el filtro activo, ○ para los demás
            let indicator = if filter == current_filter {
                "●"
            } else {
                "○"
            };
            let label = format!("{} {}", indicator, filter.label());

            StandardItem {
                label,
                activate: Box::new(move |tray: &mut PortSlayerTray| {
                    log::info!("Filtro cambiado a: {}", filter.label());
                    tray.protocol_filter = filter;
                    // Resetear a página 0 al cambiar filtro
                    tray.current_page = 0;
                }),
                ..Default::default()
            }
            .into()
        })
        .collect();

    SubMenu {
        label: format!("📊 Filtro: {}", current_filter.label()),
        submenu: submenu_items,
        ..Default::default()
    }
    .into()
}

/// Construye el submenu de tamaño de página.
///
/// Permite seleccionar entre 5 y 10 puertos por página.
///
/// # Arguments
/// * `current_size` - Tamaño de página actual
fn build_page_size_submenu(current_size: usize) -> ksni::MenuItem<PortSlayerTray> {
    let sizes: Vec<usize> = vec![5, 10];

    let submenu_items: Vec<ksni::MenuItem<PortSlayerTray>> = sizes
        .iter()
        .map(|&size| {
            let indicator = if size == current_size { "●" } else { "○" };
            let label = format!("{} {} puertos", indicator, size);

            StandardItem {
                label,
                activate: Box::new(move |tray: &mut PortSlayerTray| {
                    log::info!("Tamaño de página cambiado a: {}", size);
                    tray.page_size = size;
                    tray.current_page = 0;
                }),
                ..Default::default()
            }
            .into()
        })
        .collect();

    SubMenu {
        label: format!("📋 Por página: {}", current_size),
        submenu: submenu_items,
        ..Default::default()
    }
    .into()
}

/// Construye el item mostrado cuando no hay puertos abiertos.
fn build_empty_message() -> ksni::MenuItem<PortSlayerTray> {
    StandardItem {
        label: "✅ No hay puertos abiertos".into(),
        enabled: false,
        ..Default::default()
    }
    .into()
}

/// Construye el encabezado con el conteo de puertos.
///
/// # Arguments
/// * `total` - Total de puertos que coinciden con el filtro
/// * `filter` - Filtro activo para mostrar en la etiqueta
fn build_count_header(total: usize, filter: ProtocolFilter) -> ksni::MenuItem<PortSlayerTray> {
    let filter_label = match filter {
        ProtocolFilter::All => "".to_string(),
        _ => format!(" ({})", filter.label()),
    };

    StandardItem {
        label: format!("📡 {} puertos encontrados{}", total, filter_label),
        enabled: false,
        ..Default::default()
    }
    .into()
}

/// Construye el item "⚔️ Cerrar Todos" del menú.
///
/// # Arguments
/// * `total` - Cantidad de puertos para mostrar en la etiqueta
fn build_kill_all_item(total: usize) -> ksni::MenuItem<PortSlayerTray> {
    StandardItem {
        label: format!("⚔️ Cerrar Todos ({} puertos)", total),
        activate: Box::new(|tray: &mut PortSlayerTray| {
            log::info!("Cerrando todos los puertos...");
            match port_scanner::kill_all_port_processes() {
                Ok(count) => {
                    log::info!("{} procesos terminados", count);
                }
                Err(e) => {
                    log::error!("Error al cerrar puertos: {}", e);
                }
            }
            tray.refresh_ports();
        }),
        ..Default::default()
    }
    .into()
}

/// Construye un item individual de puerto con opción de cerrarlo.
///
/// El estilo del ícono cambia según si el proceso es conocido o no:
/// - 🔴 Puerto con PID conocido (se puede cerrar)
/// - 🟡 Puerto sin PID (desconocido, ej: Docker sin permisos)
///
/// # Arguments
/// * `port_info` - Información del puerto a mostrar
fn build_port_item(port_info: &port_scanner::PortInfo) -> ksni::MenuItem<PortSlayerTray> {
    let pid = port_info.pid;
    let port_num = port_info.port;

    // Ícono según si el PID es conocido o no
    let icon = if pid > 0 { "🔴" } else { "🟡" };

    // Etiqueta con formato: "🔴 TCP 8080 (0.0.0.0) → node [PID 1234]"
    let label = format!("{} {}", icon, port_info);

    // Habilitar botón para todos (si PID=0 usa pkexec fuser)
    let can_kill = true;
    let protocol = port_info.protocol.clone();

    StandardItem {
        label,
        enabled: can_kill,
        activate: Box::new(move |tray: &mut PortSlayerTray| {
            if pid == 0 {
                log::warn!("Puerto {} sin PID, usando fuser con pkexec", port_num);
                match port_scanner::kill_port_by_number(port_num, &protocol) {
                    Ok(()) => log::info!("Puerto {} cerrado exitosamente vía fuser", port_num),
                    Err(e) => log::error!("Error cerrando puerto {}: {}", port_num, e),
                }
            } else {
                log::info!("Cerrando puerto {} (PID: {})", port_num, pid);
                match port_scanner::kill_process(pid) {
                    Ok(()) => {
                        log::info!("Puerto {} cerrado exitosamente", port_num);
                    }
                    Err(e) => {
                        log::error!("Error cerrando puerto {}: {}", port_num, e);
                    }
                }
            }
            tray.refresh_ports();
        }),
        ..Default::default()
    }
    .into()
}

/// Construye los items de navegación entre páginas.
///
/// Genera tres items:
/// - ◀ Anterior (deshabilitado en la primera página)
/// - Página X/Y (indicador, no clickeable)
/// - ▶ Siguiente (deshabilitado en la última página)
///
/// # Arguments
/// * `current_page` - Página actual (base 0)
/// * `total_pages` - Número total de páginas
fn build_navigation_items(
    current_page: usize,
    total_pages: usize,
) -> Vec<ksni::MenuItem<PortSlayerTray>> {
    let mut items: Vec<ksni::MenuItem<PortSlayerTray>> = Vec::new();

    // Botón "Anterior"
    let can_go_prev = current_page > 0;
    items.push(
        StandardItem {
            label: "◀ Anterior".into(),
            enabled: can_go_prev,
            activate: Box::new(|tray: &mut PortSlayerTray| {
                if tray.current_page > 0 {
                    tray.current_page -= 1;
                    log::debug!("Página anterior: {}", tray.current_page + 1);
                }
            }),
            ..Default::default()
        }
        .into(),
    );

    // Indicador de página actual (no clickeable)
    items.push(
        StandardItem {
            label: format!("📄 Página {}/{}", current_page + 1, total_pages),
            enabled: false,
            ..Default::default()
        }
        .into(),
    );

    // Botón "Siguiente"
    let can_go_next = current_page + 1 < total_pages;
    items.push(
        StandardItem {
            label: "▶ Siguiente".into(),
            enabled: can_go_next,
            activate: Box::new(move |tray: &mut PortSlayerTray| {
                if tray.current_page + 1 < total_pages {
                    tray.current_page += 1;
                    log::debug!("Página siguiente: {}", tray.current_page + 1);
                }
            }),
            ..Default::default()
        }
        .into(),
    );

    items
}

/// Construye el item "❌ Salir" del menú.
fn build_exit_item() -> ksni::MenuItem<PortSlayerTray> {
    StandardItem {
        label: "❌ Salir".into(),
        activate: Box::new(|_: &mut PortSlayerTray| {
            log::info!("PortSlayer cerrándose...");
            process::exit(0);
        }),
        ..Default::default()
    }
    .into()
}

// ─────────────────────────────────────────────────────────────
// Inicio del servicio system tray
// ─────────────────────────────────────────────────────────────

/// Inicia el system tray y ejecuta el loop principal.
///
/// Crea el ícono en la bandeja del sistema y lanza un hilo de
/// actualización automática que refresca los puertos cada 10 segundos.
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

            // Notificar al tray para reconstruir el menú
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
