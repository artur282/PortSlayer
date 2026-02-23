#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use procfs::process::Process;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use sysinfo::{Pid, ProcessExt, System, SystemExt, Signal};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PortInfo {
    pub port: u16,
    pub pid: u32,
    pub name: String,
}

#[tauri::command]
fn get_active_ports() -> Vec<PortInfo> {
    let mut ports = Vec::new();
    let all_procs = match procfs::process::all_processes() {
        Ok(procs) => procs,
        Err(_) => return Vec::new(),
    };
    
    // Map inodes to PIDs and process names
    let mut inode_to_proc = HashMap::new();
    for p in all_procs {
        if let Ok(proc) = p {
            let pid = proc.pid() as u32;
            let name = proc.stat().map(|s| s.comm).unwrap_or_else(|_| "Unknown".to_string());
            if let Ok(fds) = proc.fd() {
                for fd in fds {
                    if let Ok(fd_info) = fd {
                        if let procfs::process::FDTarget::Socket(inode) = fd_info.target {
                            inode_to_proc.insert(inode, (pid, name.clone()));
                        }
                    }
                }
            }
        }
    }

    // Check TCP ports
    if let Ok(tcp) = procfs::net::tcp() {
        for entry in tcp {
            if entry.state == procfs::net::TcpState::Listen {
                let port = entry.local_address.port();
                if let Some((pid, name)) = inode_to_proc.get(&entry.inode) {
                    ports.push(PortInfo {
                        port,
                        pid: *pid,
                        name: name.clone(),
                    });
                }
            }
        }
    }

    // Check TCP6 ports
    if let Ok(tcp6) = procfs::net::tcp6() {
        for entry in tcp6 {
            if entry.state == procfs::net::TcpState::Listen {
                let port = entry.local_address.port();
                if let Some((pid, name)) = inode_to_proc.get(&entry.inode) {
                    ports.push(PortInfo {
                        port,
                        pid: *pid,
                        name: name.clone(),
                    });
                }
            }
        }
    }

    ports.sort_by_key(|p| p.port);
    ports.dedup_by_key(|p| p.port);
    ports
}

#[tauri::command]
fn kill_port_process(pid: u32) -> Result<String, String> {
    let mut system = System::new_all();
    system.refresh_all();
    
    if let Some(process) = system.process(Pid::from(pid as usize)) {
        if process.kill_with(Signal::Kill).unwrap_or(false) {
            Ok(format!("Process {} killed successfully", pid))
        } else {
            Err(format!("Failed to kill process {}", pid))
        }
    } else {
        Err(format!("Process {} not found", pid))
    }
}

#[tauri::command]
fn kill_all_ports(pids: Vec<u32>) -> Result<String, String> {
    let mut system = System::new_all();
    system.refresh_all();
    
    let mut killed_count = 0;
    for pid in pids {
        if let Some(process) = system.process(Pid::from(pid as usize)) {
            if process.kill_with(Signal::Kill).unwrap_or(false) {
                killed_count += 1;
            }
        }
    }
    
    Ok(format!("Successfully killed {} processes", killed_count))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_active_ports,
            kill_port_process,
            kill_all_ports
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
