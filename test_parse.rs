use std::process::Command;

fn extract_process_info(line: &str) -> Option<(u32, String)> {
    let users_start = line.find("users:((")?;
    let users_section = &line[users_start..];
    let name_start = users_section.find("((\"")? + 3;
    let name_end = users_section[name_start..].find('"')? + name_start;
    let process_name = users_section[name_start..name_end].to_string();
    let pid_marker = "pid=";
    let pid_start = users_section.find(pid_marker)? + pid_marker.len();
    let pid_end = users_section[pid_start..].find(|c: char| !c.is_ascii_digit()).map(|i| i + pid_start).unwrap_or(users_section.len());
    let pid: u32 = users_section[pid_start..pid_end].parse().ok()?;
    Some((pid, process_name))
}

fn extract_address_and_port(line: &str) -> Option<(String, u16)> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    for part in &parts {
        let is_address = part.contains('.') || part.contains('[') || part.contains("::") || part.starts_with('*');
        if !is_address { continue; }
        if let Some(colon_pos) = part.rfind(':') {
            let addr_part = &part[..colon_pos];
            let port_str = &part[colon_pos + 1..];
            if port_str == "*" { continue; }
            if let Ok(port) = port_str.parse::<u16>() {
                if port > 0 {
                    let cleaned = addr_part.trim_start_matches('[').trim_end_matches(']');
                    let cleaned = if let Some(pos) = cleaned.find('%') { cleaned[..pos].to_string() } else if cleaned == "*" { "0.0.0.0".to_string() } else { cleaned.to_string() };
                    return Some((cleaned, port));
                }
            }
        }
    }
    None
}

fn parse_single_ss_line(line: &str, protocol: &str) -> Option<()> {
    let line = line.trim();
    if line.is_empty() { return None; }
    let (local_address, port) = extract_address_and_port(line)?;
    let (pid, process_name) = extract_process_info(line).unwrap_or((0, "desconocido".to_string()));
    println!("Port: {}, Addr: {}, PID: {}, Name: {}", port, local_address, pid, process_name);
    Some(())
}

fn main() {
    let output = Command::new("ss").arg("-tlnpH").output().unwrap();
    let raw = String::from_utf8(output.stdout).unwrap();
    for line in raw.lines() {
        parse_single_ss_line(line, "tcp");
    }
}
