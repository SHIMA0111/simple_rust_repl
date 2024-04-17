use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

pub(crate) struct RustProject {
    crate_list: HashMap<String, String>,
    commands: Vec<String>,
}

impl RustProject {
    pub fn new() -> Self {
        RustProject {
            crate_list: HashMap::new(),
            commands: Vec::new(),
        }
    }

    pub fn add_crate(&mut self, crate_name: &str, version: &str) {
        self.crate_list.insert(crate_name.to_owned(), version.to_owned());
    }

    pub fn add_command(&mut self, command: &str) {
        self.commands.push(command.to_owned());
    }

    pub fn generate_rust(&self, temp_path: &Path) -> std::io::Result<()> {
        let mut rust_code = String::new();

        rust_code.push_str(
            &self.crate_list.iter().map(|(key, _)| format!("use {}", key)).collect::<Vec<_>>().join(";\n")
        );
        if rust_code.len() > 0 {
            rust_code.push_str("\n");
        }

        rust_code.push_str("fn main() {");

        rust_code.push_str(
            &self.commands.iter().map(|command| command.to_string()).collect::<Vec<_>>().join("\n")
        );
        rust_code.push_str("\n");
        rust_code.push_str("}");

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(temp_path)
            .unwrap();

        writeln!(file, "{}", &rust_code)
    }

    pub fn merge(&mut self, others: &Self) {
        self.commands = others.commands.clone();
        self.crate_list = others.crate_list.clone();
    }
}

