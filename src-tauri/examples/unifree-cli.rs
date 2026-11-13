//! 本地调试 / CI 用的最小 CLI（不打包进发行版）。
//!
//! 用法：
//!   cargo run --example unifree-cli -- scan
//!   cargo run --example unifree-cli -- status
//!   cargo run --example unifree-cli -- patch-hub
//!   cargo run --example unifree-cli -- restore-hub
//!   cargo run --example unifree-cli -- patch-editor <licensing-client-path> [version]
//!   cargo run --example unifree-cli -- restore-editor <licensing-client-path>
//!   cargo run --example unifree-cli -- license

use unifree_lib::{alf_generator, license, patcher, scanner, ulf_signer};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("status");

    match cmd {
        "scan" => {
            for e in scanner::scan_installed_editors() {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    e.version, e.architecture, e.dll_status, e.product_name, e.path
                );
                println!("\tlicensing: {}", e.dll_path);
            }
        }
        "status" => {
            println!("hub_install_dir : {}", scanner::hub_install_dir().display());
            println!("hub_licensing   : {}", scanner::hub_licensing_dir().display());
            println!("hub_status      : {}", patcher::get_hub_status());
            println!("license_status  : {}", license::get_ulf_status());
            println!("license_path    : {}", license::ulf_path().display());
        }
        "patch-hub" => match patcher::patch_hub(true, true) {
            Ok(msg) => println!("OK: {}", msg),
            Err(e) => {
                eprintln!("ERR: {}", e);
                std::process::exit(1);
            }
        },
        "restore-hub" => match patcher::restore_hub() {
            Ok(msg) => println!("OK: {}", msg),
            Err(e) => {
                eprintln!("ERR: {}", e);
                std::process::exit(1);
            }
        },
        "patch-editor" => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("usage: patch-editor <licensing-client-path> [version]");
                std::process::exit(2);
            });
            match patcher::patch_entitlement_resolver(path, args.get(3).map(String::as_str)) {
                Ok(msg) => println!("OK: {}", msg),
                Err(e) => {
                    eprintln!("ERR: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "restore-editor" => {
            let path = args.get(2).unwrap_or_else(|| {
                eprintln!("usage: restore-editor <licensing-client-path>");
                std::process::exit(2);
            });
            match patcher::restore(path) {
                Ok(msg) => println!("OK: {}", msg),
                Err(e) => {
                    eprintln!("ERR: {}", e);
                    std::process::exit(1);
                }
            }
        }
        "license" => {
            let alf = match alf_generator::generate_alf_file("Unity Pro") {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("ERR: {}", e);
                    std::process::exit(1);
                }
            };
            println!("ALF: {}", alf.display());
            match ulf_signer::sign_alf_to_ulf(&alf, &license::ulf_path(), None) {
                Ok(p) => println!("ULF: {}", p),
                Err(e) => {
                    eprintln!("ERR: {}", e);
                    std::process::exit(1);
                }
            }
        }
        other => {
            eprintln!("unknown command: {}", other);
            std::process::exit(2);
        }
    }
}
