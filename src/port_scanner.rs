/// Módulo de escaneo de puertos de red.
/// Lee los puertos TCP/UDP abiertos usando el comando `ss` del sistema
/// y parsea la salida para obtener información estructurada.
use std::process::Command;

/// Información de un puerto abierto en el sistema
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PortInfo {
    /// Protocolo del puerto (tcp, udp)
    pub protocol: String,
    /// Número del puerto
    pub port: u16,
    /// PID del proceso que usa el puerto
    pub pid: u32,
    /// Nombre del proceso asociado
    pub process_name: String,
}

impl std::fmt::Display for PortInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Puerto {}: {} (PID: {})",
            self.port, self.process_name, self.pid
        )
    }
}

/// Escanea los puertos TCP y UDP abiertos en el sistema.
///
/// Ejecuta `ss -tlnpH` y `ss -ulnpH` para obtener puertos en estado LISTEN.
/// Requiere permisos de root para ver los PIDs de otros usuarios.
///
/// # Returns
/// Vector con la información de cada puerto abierto encontrado.
pub fn scan_open_ports() -> Vec<PortInfo> {
    let mut ports: Vec<PortInfo> = Vec::new();

    // Escanear TCP (LISTEN) y UDP
    for (flag, protocol) in [("-tlnpH", "tcp"), ("-ulnpH", "udp")] {
        let output = execute_ss_command(flag);
        if let Some(raw_output) = output {
            let parsed = parse_ss_output(&raw_output, protocol);
            ports.extend(parsed);
        }
    }

    // Ordenar por número de puerto para consistencia visual
    ports.sort_by_key(|p| p.port);

    // Eliminar duplicados (mismo puerto y PID)
    ports.dedup_by(|a, b| a.port == b.port && a.pid == b.pid);

    ports
}

/// Ejecuta el comando `ss` con los flags indicados.
///
/// # Arguments
/// * `flags` - Flags para el comando ss (ej: "-tlnpH")
///
/// # Returns
/// `Some(String)` con la salida del comando, o `None` si falla.
fn execute_ss_command(flags: &str) -> Option<String> {
    // Intentar primero con sudo para ver PIDs de todos los procesos
    let result = Command::new("sudo")
        .args(["-n", "ss", flags])
        .output();

    match result {
        Ok(output) if output.status.success() => {
            String::from_utf8(output.stdout).ok()
        }
        _ => {
            // Fallback sin sudo (solo verá procesos propios)
            log::warn!("Ejecutando ss sin sudo - solo se verán procesos propios");
            let fallback = Command::new("ss")
                .arg(flags)
                .output()
                .ok()?;

            String::from_utf8(fallback.stdout).ok()
        }
    }
}

/// Parsea la salida del comando `ss` para extraer información de puertos.
///
/// Formato esperado de ss -tlnpH:
/// ```text
/// LISTEN  0  128  0.0.0.0:8080  0.0.0.0:*  users:(("node",pid=1234,fd=5))
/// ```
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
        .filter_map(|line| parse_single_line(line, protocol))
        .collect()
}

/// Parsea una línea individual de la salida de `ss`.
///
/// Extrae el número de puerto de la dirección local y el PID/nombre
/// del proceso de la sección "users:".
///
/// # Arguments
/// * `line` - Línea individual de la salida de ss
/// * `protocol` - Protocolo a asignar
///
/// # Returns
/// `Some(PortInfo)` si se pudo parsear exitosamente, `None` en caso contrario.
fn parse_single_line(line: &str, protocol: &str) -> Option<PortInfo> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Extraer el puerto del campo de dirección local (4to campo típicamente)
    let port = extract_port_from_line(line)?;

    // Extraer PID y nombre del proceso de la sección users:((...))
    let (pid, process_name) = extract_process_info(line)?;

    Some(PortInfo {
        protocol: protocol.to_string(),
        port,
        pid,
        process_name,
    })
}

/// Extrae el número de puerto de una línea de `ss`.
///
/// Busca el patrón `:PORT` en las direcciones locales.
/// Maneja tanto IPv4 (0.0.0.0:8080) como IPv6 ([::]:8080).
///
/// # Arguments
/// * `line` - Línea de ss con la información del socket
///
/// # Returns
/// `Some(u16)` con el número de puerto, o `None` si no se puede extraer.
fn extract_port_from_line(line: &str) -> Option<u16> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    // Formato de salida de ss -tlnpH:
    // LISTEN  0  128  0.0.0.0:8080  0.0.0.0:*  users:(("node",pid=1234,fd=5))
    // Campos: [Estado, RecvQ, SendQ, DirLocal, DirRemota, ...]
    //
    // Buscamos campos con formato de dirección IP:PUERTO (contienen '.' o '[')
    // para no confundir con valores numéricos simples como el backlog (128)
    for part in &parts {
        // Solo considerar campos que parecen direcciones (contienen . o [ para IPv6)
        let is_address = part.contains('.') || part.contains('[') || part.contains("::");
        if !is_address {
            continue;
        }

        // Extraer el puerto después del último ':'
        if let Some(port_str) = part.rsplit(':').next() {
            if port_str != "*" {
                if let Ok(port) = port_str.parse::<u16>() {
                    if port > 0 {
                        return Some(port);
                    }
                }
            }
        }
    }

    None
}

/// Extrae el PID y nombre del proceso de la sección "users:" de ss.
///
/// Busca el patrón: users:(("nombre",pid=1234,fd=5))
///
/// # Arguments
/// * `line` - Línea completa de ss
///
/// # Returns
/// Tupla (PID, nombre_proceso) si se encuentra, `None` en caso contrario.
fn extract_process_info(line: &str) -> Option<(u32, String)> {
    // Buscar la sección users:((...))
    let users_start = line.find("users:((")?;
    let users_section = &line[users_start..];

    // Extraer el nombre del proceso entre comillas
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

/// Mata un proceso por su PID usando `kill -9`.
///
/// Utiliza `pkexec` para solicitar permisos de superusuario
/// de manera gráfica al usuario.
///
/// # Arguments
/// * `pid` - ID del proceso a terminar
///
/// # Returns
/// `Ok(())` si el proceso fue terminado exitosamente,
/// `Err(String)` con el mensaje de error en caso contrario.
pub fn kill_process(pid: u32) -> Result<(), String> {
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
        // Si falla sin permisos, intentar con pkexec (prompt gráfico)
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
/// Escanea los puertos actuales y termina cada proceso encontrado.
/// Recopila PIDs únicos para no intentar matar el mismo proceso
/// varias veces.
///
/// # Returns
/// `Ok(cantidad)` con el número de procesos terminados,
/// `Err(String)` con los errores acumulados.
pub fn kill_all_port_processes() -> Result<usize, String> {
    let ports = scan_open_ports();

    if ports.is_empty() {
        return Ok(0);
    }

    // Recopilar PIDs únicos para evitar matar el mismo proceso varias veces
    let mut unique_pids: Vec<u32> = ports.iter().map(|p| p.pid).collect();
    unique_pids.sort();
    unique_pids.dedup();

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica que el parser maneja líneas vacías correctamente
    #[test]
    fn test_parse_empty_line() {
        assert!(parse_single_line("", "tcp").is_none());
        assert!(parse_single_line("   ", "tcp").is_none());
    }

    /// Verifica el parsing de una línea real de ss
    #[test]
    fn test_parse_ss_line() {
        let line = r#"LISTEN 0 128 0.0.0.0:8080 0.0.0.0:* users:(("node",pid=12345,fd=19))"#;
        let result = parse_single_line(line, "tcp");
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.port, 8080);
        assert_eq!(info.pid, 12345);
        assert_eq!(info.process_name, "node");
        assert_eq!(info.protocol, "tcp");
    }

    /// Verifica extracción de info de proceso
    #[test]
    fn test_extract_process_info() {
        let line = r#"LISTEN 0 5 127.0.0.1:5432 0.0.0.0:* users:(("postgres",pid=987,fd=3))"#;
        let (pid, name) = extract_process_info(line).unwrap();
        assert_eq!(pid, 987);
        assert_eq!(name, "postgres");
    }
}
