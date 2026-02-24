/// Módulo de escaneo de puertos de red.
///
/// Usa múltiples fuentes para garantizar la detección completa:
/// 1. Comando `ss` (fuente principal, incluye nombres de procesos)
/// 2. Archivos `/proc/net/tcp*` y `/proc/net/udp*` (fallback, detecta
///    puertos de Docker y otros que `ss` sin permisos no muestra)
///
/// Combina ambas fuentes y elimina duplicados para ofrecer una vista
/// completa de todos los puertos abiertos en el sistema.
use std::collections::HashMap;
use std::fs;
use std::process::Command;

/// Filtro de protocolo para los puertos escaneados
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProtocolFilter {
    /// Mostrar todos los protocolos
    All,
    /// Solo puertos TCP
    Tcp,
    /// Solo puertos UDP
    Udp,
}

impl ProtocolFilter {
    /// Etiqueta legible para mostrar en el menú del tray
    pub fn label(&self) -> &'static str {
        match self {
            ProtocolFilter::All => "Todos",
            ProtocolFilter::Tcp => "TCP",
            ProtocolFilter::Udp => "UDP",
        }
    }
}

/// Información de un puerto abierto en el sistema
#[derive(Debug, Clone)]
pub struct PortInfo {
    /// Protocolo del puerto (tcp, udp)
    pub protocol: String,
    /// Número del puerto
    pub port: u16,
    /// Dirección local donde escucha (ej: "0.0.0.0", "127.0.0.1", "[::]")
    pub local_address: String,
    /// PID del proceso que usa el puerto (0 si no se pudo determinar)
    pub pid: u32,
    /// Nombre del proceso asociado ("desconocido" si no se pudo determinar)
    pub process_name: String,
}

impl std::fmt::Display for PortInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Formato: "TCP 8080 (0.0.0.0) → node [PID 1234]"
        let proto_upper = self.protocol.to_uppercase();
        if self.pid > 0 {
            write!(
                f,
                "{} {} ({}) → {} [PID {}]",
                proto_upper, self.port, self.local_address, self.process_name, self.pid
            )
        } else {
            write!(
                f,
                "{} {} ({}) → {}",
                proto_upper, self.port, self.local_address, self.process_name
            )
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Escaneo principal: combina ss + /proc/net para cobertura total
// ─────────────────────────────────────────────────────────────

/// Escanea los puertos TCP y UDP abiertos en el sistema.
///
/// Usa dos fuentes de datos para cobertura completa:
/// - `ss -tlnpH` / `ss -ulnpH` → detecta PIDs si hay permisos
/// - `/proc/net/tcp*` y `/proc/net/udp*` → detecta TODOS los sockets
///   incluyendo Docker, que `ss` sin sudo no muestra con PID
///
/// Los resultados se combinan priorizando la info de `ss` (tiene PID)
/// y complementando con `/proc/net` para puertos sin PID visible.
///
/// # Returns
/// Vector ordenado por puerto con la información de cada puerto abierto.
pub fn scan_open_ports() -> Vec<PortInfo> {
    // Fase 1: Escanear con ss (incluye PIDs cuando hay permisos)
    let mut ports_map: HashMap<(String, u16), PortInfo> = HashMap::new();

    for (flag, protocol) in [("-tlnpH", "tcp"), ("-ulnpH", "udp")] {
        let output = execute_ss_command(flag);
        if let Some(raw_output) = output {
            let parsed = parse_ss_output(&raw_output, protocol);
            for port_info in parsed {
                let key = (port_info.protocol.clone(), port_info.port);
                // Priorizar entradas con PID conocido sobre las sin PID
                ports_map
                    .entry(key)
                    .and_modify(|existing| {
                        if existing.pid == 0 && port_info.pid > 0 {
                            *existing = port_info.clone();
                        }
                    })
                    .or_insert(port_info);
            }
        }
    }

    // Fase 2: Complementar con /proc/net para puertos que ss no muestra
    let proc_ports = scan_proc_net_ports();
    for port_info in proc_ports {
        let key = (port_info.protocol.clone(), port_info.port);
        // Solo insertar si no existe ya (ss tiene mejor info)
        ports_map.entry(key).or_insert(port_info);
    }

    // Convertir a vector y ordenar por número de puerto
    let mut ports: Vec<PortInfo> = ports_map.into_values().collect();
    ports.sort_by_key(|p| (p.port, p.protocol.clone()));

    log::info!("Escaneo completado: {} puertos encontrados", ports.len());
    ports
}

/// Filtra una lista de puertos según el filtro de protocolo.
///
/// # Arguments
/// * `ports` - Referencia a los puertos a filtrar
/// * `filter` - Filtro de protocolo a aplicar
///
/// # Returns
/// Vector filtrado con solo los puertos que coinciden con el filtro.
pub fn filter_ports(ports: &[PortInfo], filter: ProtocolFilter) -> Vec<PortInfo> {
    match filter {
        ProtocolFilter::All => ports.to_vec(),
        ProtocolFilter::Tcp => ports
            .iter()
            .filter(|p| p.protocol == "tcp")
            .cloned()
            .collect(),
        ProtocolFilter::Udp => ports
            .iter()
            .filter(|p| p.protocol == "udp")
            .cloned()
            .collect(),
    }
}

/// Calcula el número total de páginas para la paginación.
///
/// # Arguments
/// * `total_items` - Cantidad total de elementos
/// * `page_size` - Elementos por página
///
/// # Returns
/// Número total de páginas (mínimo 1).
pub fn total_pages(total_items: usize, page_size: usize) -> usize {
    if total_items == 0 || page_size == 0 {
        return 1;
    }
    total_items.div_ceil(page_size)
}

/// Obtiene una página de puertos para mostrar en el menú.
///
/// # Arguments
/// * `ports` - Lista completa de puertos (ya filtrados)
/// * `page` - Número de página (base 0)
/// * `page_size` - Cantidad de puertos por página
///
/// # Returns
/// Slice del vector correspondiente a la página solicitada.
pub fn get_page(ports: &[PortInfo], page: usize, page_size: usize) -> Vec<PortInfo> {
    if page_size == 0 {
        return Vec::new();
    }
    let start = page * page_size;
    if start >= ports.len() {
        return Vec::new();
    }
    let end = (start + page_size).min(ports.len());
    ports[start..end].to_vec()
}

// ─────────────────────────────────────────────────────────────
// Fuente 1: Comando `ss` del sistema
// ─────────────────────────────────────────────────────────────

/// Ejecuta el comando `ss` con los flags indicados.
///
/// Intenta primero con `sudo -n` (sin password) para ver PIDs de
/// todos los procesos. Si falla, ejecuta sin sudo como fallback.
///
/// # Arguments
/// * `flags` - Flags para el comando ss (ej: "-tlnpH")
///
/// # Returns
/// `Some(String)` con la salida del comando, o `None` si falla.
fn execute_ss_command(flags: &str) -> Option<String> {
    // Intentar primero con sudo para ver PIDs de todos los procesos
    let result = Command::new("sudo").args(["-n", "ss", flags]).output();

    match result {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout).ok(),
        _ => {
            // Fallback sin sudo (solo verá procesos propios)
            log::warn!("Ejecutando ss sin sudo - algunos PIDs no serán visibles");
            let fallback = Command::new("ss").arg(flags).output().ok()?;

            String::from_utf8(fallback.stdout).ok()
        }
    }
}

/// Parsea la salida del comando `ss` para extraer información de puertos.
///
/// Ahora acepta líneas SIN información de proceso (users:((...))),
/// asignando PID=0 y nombre="desconocido" para esos puertos.
/// Esto es crucial para detectar puertos de Docker y otros servicios
/// del sistema que no muestran PID sin privilegios de root.
///
/// # Arguments
/// * `output` - Salida cruda del comando ss
/// * `protocol` - Protocolo a asignar ("tcp" o "udp")
///
/// # Returns
/// Vector con la información parseada de cada puerto.
fn parse_ss_output(output: &str, protocol: &str) -> Vec<PortInfo> {
    output
        .lines()
        .filter_map(|line| parse_single_ss_line(line, protocol))
        .collect()
}

/// Parsea una línea individual de la salida de `ss`.
///
/// Extrae el puerto y la dirección local. Si hay sección `users:((...))`
/// extrae PID y nombre del proceso; si no, usa valores por defecto.
///
/// Formato esperado de ss -tlnpH:
/// ```text
/// LISTEN  0  128  0.0.0.0:8080  0.0.0.0:*  users:(("node",pid=1234,fd=5))
/// LISTEN  0  4096       *:8069        *:*
/// ```
///
/// # Arguments
/// * `line` - Línea individual de la salida de ss
/// * `protocol` - Protocolo a asignar
///
/// # Returns
/// `Some(PortInfo)` si se pudo parsear exitosamente, `None` si la línea
/// es vacía o no contiene información de puerto válida.
fn parse_single_ss_line(line: &str, protocol: &str) -> Option<PortInfo> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Extraer dirección local y puerto
    let (local_address, port) = extract_address_and_port(line)?;

    // Extraer PID y nombre del proceso (OPCIONAL - puede no existir)
    let (pid, process_name) = extract_process_info(line).unwrap_or((0, "desconocido".to_string()));

    Some(PortInfo {
        protocol: protocol.to_string(),
        port,
        local_address,
        pid,
        process_name,
    })
}

/// Extrae la dirección local y el número de puerto de una línea de `ss`.
///
/// Maneja múltiples formatos de dirección:
/// - IPv4: `0.0.0.0:8080`, `127.0.0.1:5432`
/// - IPv6: `[::]:8080`, `[::1]:631`
/// - Wildcard: `*:8069`
///
/// # Arguments
/// * `line` - Línea de ss con la información del socket
///
/// # Returns
/// Tupla `(dirección_local, puerto)` o `None` si no se puede extraer.
fn extract_address_and_port(line: &str) -> Option<(String, u16)> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    // Formato ss: [Estado, RecvQ, SendQ, DirLocal, DirRemota, ...]
    // DirLocal puede ser: "0.0.0.0:8080", "[::]:8080", "*:8069",
    //                     "127.0.0.53%lo:53"
    for part in &parts {
        // Identificar campos que parecen direcciones de socket
        let is_address = part.contains('.')
            || part.contains('[')
            || part.contains("::")
            || part.starts_with('*');

        if !is_address {
            continue;
        }

        // Extraer dirección y puerto después del último ':'
        if let Some(colon_pos) = part.rfind(':') {
            let addr_part = &part[..colon_pos];
            let port_str = &part[colon_pos + 1..];

            // Ignorar el campo de dirección remota (contiene '*')
            if port_str == "*" {
                continue;
            }

            if let Ok(port) = port_str.parse::<u16>() {
                if port > 0 {
                    // Limpiar la dirección para presentación
                    let clean_addr = clean_address(addr_part);
                    return Some((clean_addr, port));
                }
            }
        }
    }

    None
}

/// Limpia una dirección de red para presentación legible.
///
/// Remueve decoradores como corchetes IPv6 y sufijos de interfaz (%lo).
///
/// # Arguments
/// * `addr` - Dirección cruda del socket
///
/// # Returns
/// String con la dirección limpia para mostrar al usuario.
fn clean_address(addr: &str) -> String {
    let cleaned = addr.trim_start_matches('[').trim_end_matches(']');

    // Remover sufijo de interfaz (ej: "127.0.0.53%lo" → "127.0.0.53")
    if let Some(pos) = cleaned.find('%') {
        cleaned[..pos].to_string()
    } else if cleaned == "*" {
        "0.0.0.0".to_string()
    } else {
        cleaned.to_string()
    }
}

/// Extrae el PID y nombre del proceso de la sección "users:" de ss.
///
/// Busca el patrón: `users:(("nombre",pid=1234,fd=5))`
///
/// # Arguments
/// * `line` - Línea completa de ss
///
/// # Returns
/// Tupla (PID, nombre_proceso) si se encuentra, `None` si la línea
/// no contiene información de proceso.
fn extract_process_info(line: &str) -> Option<(u32, String)> {
    // Buscar la sección users:((...)
    let users_start = line.find("users:((")?;
    let users_section = &line[users_start..];

    // Extraer el nombre del proceso entre comillas: (("nombre"
    let name_start = users_section.find("((\"")? + 3;
    let name_end = users_section[name_start..].find('"')? + name_start;
    let process_name = users_section[name_start..name_end].to_string();

    // Extraer el PID del patrón pid=NUMERO
    let pid_marker = "pid=";
    let pid_start = users_section.find(pid_marker)? + pid_marker.len();
    let pid_end = users_section[pid_start..]
        .find(|c: char| !c.is_ascii_digit())
        .map(|i| i + pid_start)
        .unwrap_or(users_section.len());
    let pid: u32 = users_section[pid_start..pid_end].parse().ok()?;

    Some((pid, process_name))
}

// ─────────────────────────────────────────────────────────────
// Fuente 2: /proc/net/* (detecta Docker y sockets sin PID visible)
// ─────────────────────────────────────────────────────────────

/// Escanea puertos desde los archivos /proc/net/ del kernel.
///
/// Lee `/proc/net/tcp`, `/proc/net/tcp6`, `/proc/net/udp`, `/proc/net/udp6`
/// para encontrar sockets en estado LISTEN (0x0A para TCP) o abiertos (UDP).
/// Esta fuente siempre está disponible y detecta TODOS los sockets,
/// incluyendo los de Docker, independientemente de los permisos.
///
/// # Returns
/// Vector con los puertos encontrados. PID y nombre serán 0/"desconocido"
/// a menos que se pueda determinar escaneando /proc/[pid]/fd.
fn scan_proc_net_ports() -> Vec<PortInfo> {
    let mut ports: Vec<PortInfo> = Vec::new();

    // Mapeo inode→PID para intentar resolver procesos
    let inode_to_pid = build_inode_pid_map();

    // Archivos /proc/net a leer con su protocolo correspondiente
    let proc_files = [
        ("/proc/net/tcp", "tcp"),
        ("/proc/net/tcp6", "tcp"),
        ("/proc/net/udp", "udp"),
        ("/proc/net/udp6", "udp"),
    ];

    for (path, protocol) in &proc_files {
        if let Ok(content) = fs::read_to_string(path) {
            let parsed = parse_proc_net_file(&content, protocol, &inode_to_pid);
            ports.extend(parsed);
        }
    }

    ports
}

/// Parsea un archivo /proc/net/tcp o similar.
///
/// Formato de cada línea (después del header):
/// ```text
///   sl  local_address rem_address   st tx_queue rx_queue ...  inode
///    0: 00000000:0BB8 00000000:0000 0A ...                    22881
/// ```
///
/// Campos relevantes:
/// - Campo 1 (local_address): dirección IP en hex + puerto hex
/// - Campo 3 (st): estado del socket (0A = LISTEN para TCP)
/// - Campo 9 (inode): inode del socket para resolver PID
///
/// # Arguments
/// * `content` - Contenido del archivo /proc/net/*
/// * `protocol` - Protocolo ("tcp" o "udp")
/// * `inode_to_pid` - Mapa de inode a (PID, nombre_proceso)
///
/// # Returns
/// Vector de PortInfo para cada socket en estado LISTEN.
fn parse_proc_net_file(
    content: &str,
    protocol: &str,
    inode_to_pid: &HashMap<u64, (u32, String)>,
) -> Vec<PortInfo> {
    content
        .lines()
        .skip(1) // Saltar el header
        .filter_map(|line| parse_proc_net_line(line, protocol, inode_to_pid))
        .collect()
}

/// Parsea una línea individual de /proc/net/tcp o similar.
///
/// # Arguments
/// * `line` - Línea del archivo /proc/net/*
/// * `protocol` - Protocolo a asignar
/// * `inode_to_pid` - Mapa para resolver inode → PID
///
/// # Returns
/// `Some(PortInfo)` si es un socket en LISTEN, `None` en caso contrario.
fn parse_proc_net_line(
    line: &str,
    protocol: &str,
    inode_to_pid: &HashMap<u64, (u32, String)>,
) -> Option<PortInfo> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 10 {
        return None;
    }

    // Campo 3 (índice 3): estado del socket
    // 0A = LISTEN (TCP), 07 = CLOSE (UDP no tiene LISTEN, pero
    // los sockets UDP se consideran "abiertos")
    let state = parts[3];

    // Para TCP solo nos interesan los que están en LISTEN (0A)
    // Para UDP aceptamos cualquier estado (07 = CLOSE es normal)
    if protocol == "tcp" && state != "0A" {
        return None;
    }
    // Para UDP, filtrar estados no relevantes
    if protocol == "udp" && state != "07" {
        return None;
    }

    // Campo 1 (índice 1): dirección local en formato HEX:PORT_HEX
    let local_addr_raw = parts[1];
    let (local_address, port) = parse_hex_address(local_addr_raw)?;

    // Ignorar puertos 0 (sockets no enlazados)
    if port == 0 {
        return None;
    }

    // Campo 9 (índice 9): inode del socket
    let inode: u64 = parts[9].parse().unwrap_or(0);

    // Intentar resolver PID y nombre del proceso usando el inode
    let (pid, process_name) = if inode > 0 {
        inode_to_pid
            .get(&inode)
            .cloned()
            .unwrap_or((0, "desconocido".to_string()))
    } else {
        (0, "desconocido".to_string())
    };

    Some(PortInfo {
        protocol: protocol.to_string(),
        port,
        local_address,
        pid,
        process_name,
    })
}

/// Convierte una dirección hexadecimal de /proc/net a formato legible.
///
/// Formato de entrada: `HEX_IP:HEX_PORT`
/// - IPv4: `00000000:0BB8` → ("0.0.0.0", 3000)
/// - IPv6: `00000000000000000000000000000000:0BB8` → ("::", 3000)
///
/// # Arguments
/// * `hex_addr` - Dirección en formato hexadecimal de /proc/net
///
/// # Returns
/// Tupla `(dirección_legible, puerto)` o `None` si el formato es inválido.
fn parse_hex_address(hex_addr: &str) -> Option<(String, u16)> {
    let parts: Vec<&str> = hex_addr.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    // Parsear el puerto (siempre es hex de 4 caracteres)
    let port = u16::from_str_radix(parts[1], 16).ok()?;

    // Parsear la dirección IP
    let addr_hex = parts[0];
    let address = if addr_hex.len() == 8 {
        // IPv4: bytes en orden inverso (little-endian)
        let ip = u32::from_str_radix(addr_hex, 16).ok()?;
        format!(
            "{}.{}.{}.{}",
            ip & 0xff,
            (ip >> 8) & 0xff,
            (ip >> 16) & 0xff,
            (ip >> 24) & 0xff,
        )
    } else if addr_hex.len() == 32 {
        // IPv6: simplificar para la presentación
        if addr_hex == "00000000000000000000000000000000" {
            "[::]".to_string()
        } else if addr_hex == "00000000000000000000000001000000" {
            "[::1]".to_string()
        } else {
            // Mostrar versión abreviada para otras direcciones IPv6
            format!("[{}...{}]", &addr_hex[..4], &addr_hex[28..])
        }
    } else {
        return None;
    };

    Some((address, port))
}

/// Construye un mapa de inode → (PID, nombre_proceso).
///
/// Escanea `/proc/[pid]/fd/` buscando symlinks a `socket:[inode]`
/// para poder resolver qué proceso posee cada socket.
///
/// Solo escanea procesos accesibles para el usuario actual.
///
/// # Returns
/// HashMap donde la clave es el inode del socket y el valor
/// es la tupla (PID, nombre del proceso).
fn build_inode_pid_map() -> HashMap<u64, (u32, String)> {
    let mut map: HashMap<u64, (u32, String)> = HashMap::new();

    // Listar todos los directorios numéricos en /proc (cada uno es un PID)
    let proc_dir = match fs::read_dir("/proc") {
        Ok(dir) => dir,
        Err(_) => return map,
    };

    for entry in proc_dir.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Solo directorios numéricos (PIDs)
        let pid: u32 = match name_str.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Leer el nombre del proceso desde /proc/[pid]/comm
        let process_name = read_process_name(pid);

        // Escanear los file descriptors buscando sockets
        let fd_path = format!("/proc/{}/fd", pid);
        let fd_dir = match fs::read_dir(&fd_path) {
            Ok(dir) => dir,
            Err(_) => continue,
        };

        for fd_entry in fd_dir.flatten() {
            // Leer el symlink del FD (ej: "socket:[22881]")
            if let Ok(link) = fs::read_link(fd_entry.path()) {
                let link_str = link.to_string_lossy().to_string();
                if let Some(inode) = extract_socket_inode(&link_str) {
                    map.insert(inode, (pid, process_name.clone()));
                }
            }
        }
    }

    map
}

/// Lee el nombre del proceso desde /proc/[pid]/comm.
///
/// # Arguments
/// * `pid` - ID del proceso
///
/// # Returns
/// Nombre del proceso o "desconocido" si no se puede leer.
fn read_process_name(pid: u32) -> String {
    let comm_path = format!("/proc/{}/comm", pid);
    fs::read_to_string(comm_path)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "desconocido".to_string())
}

/// Extrae el inode de un symlink con formato `socket:[INODE]`.
///
/// # Arguments
/// * `link` - Contenido del symlink (ej: "socket:[22881]")
///
/// # Returns
/// `Some(inode)` si el formato es correcto, `None` en caso contrario.
fn extract_socket_inode(link: &str) -> Option<u64> {
    if link.starts_with("socket:[") && link.ends_with(']') {
        let inode_str = &link[8..link.len() - 1];
        inode_str.parse().ok()
    } else {
        None
    }
}

// ─────────────────────────────────────────────────────────────
// Acciones sobre procesos: kill individual y masivo
// ─────────────────────────────────────────────────────────────

/// Mata un proceso por su PID usando `kill -9`.
///
/// Primero intenta sin privilegios elevados. Si falla, usa `pkexec`
/// para solicitar permisos de superusuario de manera gráfica.
///
/// # Arguments
/// * `pid` - ID del proceso a terminar (debe ser > 0)
///
/// # Returns
/// `Ok(())` si el proceso fue terminado exitosamente,
/// `Err(String)` con el mensaje de error en caso contrario.
pub fn kill_process(pid: u32) -> Result<(), String> {
    if pid == 0 {
        return Err("No se puede matar un proceso con PID desconocido (0)".to_string());
    }

    log::info!("Intentando matar proceso con PID: {}", pid);

    // Usar kill con señal SIGKILL (9) para forzar cierre
    let result = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .output()
        .map_err(|e| format!("Error ejecutando kill: {}", e))?;

    if result.status.success() {
        log::info!("Proceso {} terminado exitosamente", pid);
        Ok(())
    } else {
        // Fallback con pkexec para permisos elevados (prompt gráfico)
        log::warn!("Kill sin permisos falló, intentando con pkexec...");
        let elevated = Command::new("pkexec")
            .args(["kill", "-9", &pid.to_string()])
            .output()
            .map_err(|e| format!("Error ejecutando pkexec: {}", e))?;

        if elevated.status.success() {
            log::info!("Proceso {} terminado con permisos elevados", pid);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&elevated.stderr);
            Err(format!("No se pudo matar el proceso {}: {}", pid, stderr))
        }
    }
}

/// Mata todos los procesos asociados a puertos abiertos.
///
/// Escanea los puertos actuales, recopila PIDs únicos (excluyendo
/// PID=0 que son procesos desconocidos), y los termina uno a uno.
///
/// # Returns
/// `Ok(cantidad)` con el número de procesos terminados exitosamente,
/// `Err(String)` con errores acumulados si todos fallan.
pub fn kill_all_port_processes() -> Result<usize, String> {
    let ports = scan_open_ports();

    if ports.is_empty() {
        return Ok(0);
    }

    // Recopilar PIDs únicos, excluyendo PID 0 (desconocidos)
    let mut unique_pids: Vec<u32> = ports.iter().map(|p| p.pid).filter(|pid| *pid > 0).collect();
    unique_pids.sort();
    unique_pids.dedup();

    if unique_pids.is_empty() {
        return Err("No hay procesos con PID conocido que cerrar".to_string());
    }

    let mut killed_count = 0;
    let mut errors: Vec<String> = Vec::new();

    for pid in &unique_pids {
        match kill_process(*pid) {
            Ok(()) => killed_count += 1,
            Err(e) => errors.push(e),
        }
    }

    if errors.is_empty() {
        Ok(killed_count)
    } else if killed_count > 0 {
        log::warn!(
            "Se mataron {} procesos, pero hubo errores: {:?}",
            killed_count,
            errors
        );
        Ok(killed_count)
    } else {
        Err(errors.join("; "))
    }
}

// ─────────────────────────────────────────────────────────────
// Tests unitarios
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que el parser maneja líneas vacías correctamente
    #[test]
    fn test_parse_empty_line() {
        assert!(parse_single_ss_line("", "tcp").is_none());
        assert!(parse_single_ss_line("   ", "tcp").is_none());
    }

    /// Verifica el parsing de una línea con info de proceso
    #[test]
    fn test_parse_ss_line_with_process() {
        let line = r#"LISTEN 0 128 0.0.0.0:8080 0.0.0.0:* users:(("node",pid=12345,fd=19))"#;
        let result = parse_single_ss_line(line, "tcp");
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.port, 8080);
        assert_eq!(info.pid, 12345);
        assert_eq!(info.process_name, "node");
        assert_eq!(info.protocol, "tcp");
        assert_eq!(info.local_address, "0.0.0.0");
    }

    /// Verifica el parsing de una línea SIN info de proceso (caso Docker)
    #[test]
    fn test_parse_ss_line_without_process() {
        let line = "LISTEN 0 4096       *:8069        *:*";
        let result = parse_single_ss_line(line, "tcp");
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.port, 8069);
        assert_eq!(info.pid, 0);
        assert_eq!(info.process_name, "desconocido");
    }

    /// Verifica parsing de línea con wildcard IPv4/IPv6
    #[test]
    fn test_parse_ss_wildcard_address() {
        let line = "LISTEN 0 4096  0.0.0.0:3000  0.0.0.0:*";
        let result = parse_single_ss_line(line, "tcp");
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.port, 3000);
        assert_eq!(info.local_address, "0.0.0.0");
    }

    /// Verifica extracción de info de proceso
    #[test]
    fn test_extract_process_info() {
        let line = r#"LISTEN 0 5 127.0.0.1:5432 0.0.0.0:* users:(("postgres",pid=987,fd=3))"#;
        let (pid, name) = extract_process_info(line).unwrap();
        assert_eq!(pid, 987);
        assert_eq!(name, "postgres");
    }

    /// Verifica que extract_process_info retorna None sin sección users
    #[test]
    fn test_extract_process_info_none() {
        let line = "LISTEN 0 4096  *:8069  *:*";
        assert!(extract_process_info(line).is_none());
    }

    /// Verifica conversión de dirección hex IPv4
    #[test]
    fn test_parse_hex_address_ipv4() {
        // 00000000:0BB8 = 0.0.0.0:3000
        let (addr, port) = parse_hex_address("00000000:0BB8").unwrap();
        assert_eq!(port, 3000);
        assert_eq!(addr, "0.0.0.0");
    }

    /// Verifica conversión de dirección hex IPv4 loopback
    #[test]
    fn test_parse_hex_address_loopback() {
        // 0100007F:1538 = 127.0.0.1:5432
        let (addr, port) = parse_hex_address("0100007F:1538").unwrap();
        assert_eq!(port, 5432);
        assert_eq!(addr, "127.0.0.1");
    }

    /// Verifica extracción de inode de socket
    #[test]
    fn test_extract_socket_inode() {
        assert_eq!(extract_socket_inode("socket:[22881]"), Some(22881));
        assert_eq!(extract_socket_inode("pipe:[123]"), None);
        assert_eq!(extract_socket_inode("anon_inode:"), None);
    }

    /// Verifica el filtrado por protocolo
    #[test]
    fn test_filter_ports() {
        let ports = vec![
            PortInfo {
                protocol: "tcp".into(),
                port: 80,
                local_address: "0.0.0.0".into(),
                pid: 1,
                process_name: "nginx".into(),
            },
            PortInfo {
                protocol: "udp".into(),
                port: 53,
                local_address: "0.0.0.0".into(),
                pid: 2,
                process_name: "dnsmasq".into(),
            },
        ];

        assert_eq!(filter_ports(&ports, ProtocolFilter::Tcp).len(), 1);
        assert_eq!(filter_ports(&ports, ProtocolFilter::Udp).len(), 1);
        assert_eq!(filter_ports(&ports, ProtocolFilter::All).len(), 2);
    }

    /// Verifica la paginación
    #[test]
    fn test_pagination() {
        let ports: Vec<PortInfo> = (1..=25)
            .map(|i| PortInfo {
                protocol: "tcp".into(),
                port: i as u16,
                local_address: "0.0.0.0".into(),
                pid: i,
                process_name: format!("proc{}", i),
            })
            .collect();

        // 25 items, 10 por página = 3 páginas
        assert_eq!(total_pages(25, 10), 3);

        // Página 0: puertos 1-10
        let page0 = get_page(&ports, 0, 10);
        assert_eq!(page0.len(), 10);
        assert_eq!(page0[0].port, 1);

        // Página 2: puertos 21-25
        let page2 = get_page(&ports, 2, 10);
        assert_eq!(page2.len(), 5);
        assert_eq!(page2[0].port, 21);

        // Página fuera de rango
        let page_oob = get_page(&ports, 5, 10);
        assert!(page_oob.is_empty());
    }

    /// Verifica limpieza de direcciones
    #[test]
    fn test_clean_address() {
        assert_eq!(clean_address("[::1]"), "::1");
        assert_eq!(clean_address("127.0.0.53%lo"), "127.0.0.53");
        assert_eq!(clean_address("*"), "0.0.0.0");
        assert_eq!(clean_address("0.0.0.0"), "0.0.0.0");
    }
}
