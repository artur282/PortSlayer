mod port_scanner;

fn main() {
    let ports = port_scanner::scan_open_ports();
    for p in ports {
        println!("{}", p);
    }
}
