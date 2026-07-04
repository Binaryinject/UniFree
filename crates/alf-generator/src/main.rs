//! ALF Generator CLI
//!
//! 命令行工具，用于生成 Unity ALF 文件

use std::path::PathBuf;
use alf_generator::AlfGenerator;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut output_path = PathBuf::from("Unity_lic.alf");
    let mut unity_version = "2017.2.0".to_string();
    let mut show_bindings = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_path = PathBuf::from(&args[i + 1]);
                    i += 2;
                } else {
                    eprintln!("Error: --output requires a path argument");
                    std::process::exit(1);
                }
            }
            "-v" | "--version" => {
                if i + 1 < args.len() {
                    unity_version = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: --version requires a version argument");
                    std::process::exit(1);
                }
            }
            "-b" | "--bindings" => {
                show_bindings = true;
                i += 1;
            }
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                print_help();
                std::process::exit(1);
            }
        }
    }

    let generator = AlfGenerator::new().with_unity_version(&unity_version);

    if show_bindings {
        println!("Generating ALF with bindings:");
        println!("  Unity Version: {}", unity_version);
        println!();
    }

    let alf_content = generator.generate();

    match std::fs::write(&output_path, &alf_content) {
        Ok(_) => {
            println!("ALF file generated: {}", output_path.display());
            if show_bindings {
                println!();
                println!("Content:");
                println!("{}", alf_content);
            }
        }
        Err(e) => {
            eprintln!("Error writing ALF file: {}", e);
            std::process::exit(1);
        }
    }
}

fn print_help() {
    println!("ALF Generator - Unity Activation License File Generator");
    println!();
    println!("USAGE:");
    println!("    alf-gen [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -o, --output <PATH>    Output file path (default: Unity_lic.alf)");
    println!("    -v, --version <VER>    Unity version (default: 2017.2.0)");
    println!("    -b, --bindings         Show machine bindings in output");
    println!("    -h, --help             Print this help message");
    println!();
    println!("EXAMPLES:");
    println!("    alf-gen");
    println!("    alf-gen -o my_license.alf");
    println!("    alf-gen -v 2022.3.0f1 -b");
}
