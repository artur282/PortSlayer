mod port_scanner;
fn main() {
    let ports = port_scanner::scan_open_ports();
    for p in ports {
        println!("Port: {} PID: {} Name: {}", p.port, p.pid, p.process_name);
    }
}
