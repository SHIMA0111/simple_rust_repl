mod generator;

use std::fs::create_dir;
use std::io::Write;
use std::path::Path;
use std::process::{Command, exit};
use std::str::from_utf8;
use regex::Regex;
use tempfile::NamedTempFile;
use crate::generator::RustProject;

const REGEX_VER_PATTERN: &str = r"\d+.\d+.\d+";
const REGEX_DATE_PATTERN: &str = r"\d+-\d+-\d+";
const CACHE_DIR: &str = "./.rust_repl";

fn main() {
    let regex_version = Regex::new(REGEX_VER_PATTERN).unwrap_or_else(|e| {
        eprintln!("regex version format is invalid please contact the developer");
        exit(1);
    });
    let regex_date = Regex::new(REGEX_DATE_PATTERN).unwrap_or_else(|e| {
        eprintln!("regex date format is invalid please contact the developer");
        exit(1);
    });

    let cache_path = Path::new(CACHE_DIR);
    if !cache_path.exists() {
        if let Err(e) = create_dir(cache_path) {
            eprintln!("Failed to create cache dir due to {}", e.to_string());
            exit(1);
        }
    }
    let history_path = cache_path.join("history.log");

    let tmp_file = match NamedTempFile::new() {
        Ok(tmp_file) => tmp_file,
        Err(e) => {
            eprintln!("Internal temp file creation failed due to {}", e.to_string());
            exit(1);
        }
    };

    let tmp_file_path = tmp_file.path();

    let cargo = match Command::new("cargo").arg("--version").output() {
        Ok(output) => {
            let std_out = match from_utf8(&output.stdout) {
                Ok(value) => value.to_string(),
                Err(_) => String::from_utf8_lossy(&output.stdout).to_string(),
            };
            assert!(std_out.starts_with("cargo"));

            std_out
        }
        Err(_) => {
            eprintln!(
                "Your computer doesn't have cargo which is Rust ecosystem. \
                Please install cargo first by following \
                https://doc.rust-lang.org/cargo/getting-started/installation.html");
            exit(1);
        }
    };
    let cargo_version = match regex_version.find(&cargo) {
        Some(version) => version.as_str().to_owned(),
        None => "unknown".to_string(),
    };
    let cargo_release_date = match regex_date.find(&cargo) {
        Some(date) => date.as_str().to_owned(),
        None => "unknown".to_string(),
    };

    println!("Cargo {} [Released at {}] on {}", cargo_version, cargo_release_date, std::env::consts::OS);
    println!("Please enter 'exit' when you finish interactive rust!\n");

    let mut editor = match rustyline::DefaultEditor::new() {
        Ok(editor) => editor,
        Err(e) => {
            eprintln!("Generate cli editor failed due to {}", e.to_string());
            exit(1);
        }
    };

    if history_path.exists() {
        if let Err(_) = editor.load_history(&history_path) {
            eprintln!("Failed to load history of commands");
        };
    }

    let mut backup_rust_code = RustProject::new();
    let mut unstable_rust_code = RustProject::new();

    loop {
        let readline = match editor.readline("rust>> ") {
            Ok(input) => {
                if let Err(_) = editor.add_history_entry(&input) {
                    eprintln!("Failed to save history of your input.");
                };
                input
            },
            Err(_) => continue,
        };

        if readline.is_empty() {
            continue;
        }

        let input_trim = readline.trim().to_owned();

        if input_trim == "exit" {
            println!("Bye...");
            editor.save_history(&history_path).unwrap_or_else(|e| {
                eprintln!("Failed to save history of commands due to {}", e.to_string());
            });
            break;
        }

        if !input_trim.ends_with(";") {
            eprintln!("Syntax error: {}", input_trim);
            eprintln!("Rust command should be ended with ';'.");
            continue;
        }


        if input_trim.starts_with("use") {
            if input_trim.len() <= 4 ||
                input_trim.split_whitespace().collect::<Vec<_>>().len() < 2 {
                eprintln!("use statement for using crate should have followed 'use CRATE' format.\
                 Please try again.");
                continue;
            }
            let crate_name = input_trim[4..].to_string();
            println!("{}", crate_name);

            let version = if !crate_name.starts_with("std::") {
                match editor.readline("What version you want to use?: ") {
                    Ok(input) => {
                        let regex = Regex::new(r"\d+|\d+.\d+|\d+.\d+.\d+").unwrap();

                        match regex.find(&input) {
                            Some(value) => value.as_str().to_owned(),
                            None => {
                                eprintln!("If you want to use external crate, you need to valid version. Please try again.");
                                continue;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Cannot read your input by {}. Please try again.", e.to_string());
                        continue;
                    }
                }
            } else {
                "Default".to_string()
            };
            unstable_rust_code.add_crate(&input_trim[4..], &version);
        }
        else if input_trim.starts_with("const") || input_trim.starts_with("static") {
            eprintln!("Current this Rust REPL tool can't support 'const' and 'static' definition. Please use 'let' instead of these.");
            continue;
        }
        else {
            unstable_rust_code.add_command(&input_trim);
        }

        if input_trim.starts_with("let") {
            backup_rust_code.merge(&unstable_rust_code);
            continue;
        }

        if let Err(e) = unstable_rust_code.generate_rust(tmp_file_path) {
            eprintln!("Failed to generate rust source code due to {}", e.to_string());
            exit(1);
        }

        let output = match Command::new("rustc")
            .arg(tmp_file_path)
            .arg("--crate-name")
            .arg("temp_rust")
            .arg("-o")
            .arg(tmp_file_path.parent().unwrap().join("temp_exe"))
            .output() {
            Ok(output_value) => output_value,
            Err(e) => {
                eprintln!("Unexpected Error occurred due to {}",e);
                exit(1);
            }
        };
        if !output.stderr.is_empty() {
            let std_err = String::from_utf8_lossy(&output.stderr).to_string();
            if std_err.starts_with("warning") {
                if !std_err.contains("unused import") || !std_err.contains("unused variable") {
                    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                }
                backup_rust_code.merge(&unstable_rust_code);
            }
            else {
                eprintln!("compiler return error: {}", String::from_utf8_lossy(&output.stderr));
                unstable_rust_code.merge(&backup_rust_code);
            }
        }
        else {
            let res = match Command::new(tmp_file_path.parent().unwrap().join("temp_exe")).output() {
                Ok(res) => res,
                Err(e) => {
                    eprintln!("Running native code error due to {}", e.to_string());
                    continue;
                }
            };

            println!("{}", String::from_utf8_lossy(&res.stdout));
            backup_rust_code.merge(&unstable_rust_code);
        }
    }
}
