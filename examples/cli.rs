use procfs::net::TcpNetEntry;
use procfs::process::Process;
use std::collections::HashMap;

fn main() {
    println!("--- Puertos Activos (CLI) ---");
    let mut inode_to_proc = HashMap::new();
    if let Ok(all_procs) = procfs::process::all_processes() {
        for p in all_procs {
            if let Ok(proc) = p {
                let pid = proc.pid();
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
    }

    if let Ok(tcp) = procfs::net::tcp() {
        for entry in tcp {
            if entry.state == procfs::net::TcpState::Listen {
                let port = entry.local_address.port();
                if let Some((pid, name)) = inode_to_proc.get(&entry.inode) {
                    println!("Puerto {}: {} (PID {})", port, name, pid);
                }
            }
        }
    }
}
