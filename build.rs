use std::process::Command;

fn main() {
    if cfg!(target_os = "windows") {
        let output = Command::new("cmd")
            .args(&["/C", "where dart.lib"])
            .output()
            .expect("Could not execute where command");

        let path = String::from_utf8(output.stdout).unwrap();

        println!("cargo:rustc-link-search={}", std::path::Path::new(&path).parent().unwrap().display());
    }
}