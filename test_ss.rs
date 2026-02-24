use std::process::Command;
fn main() {
    let output = Command::new("ss").arg("-tlnpH").output().unwrap();
    println!("{}", String::from_utf8(output.stdout).unwrap());
}
