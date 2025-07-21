use protobuf_codegen::Codegen;
use std::fs;
use std::path::Path;
use std::env;

fn main() {
    // Build protobuf files
    Codegen::new()
        .pure()
        .cargo_out_dir("protos")
        .input("src/protos/Mumble.proto")
        .include("src/protos")
        .run_from_script();

    // Generate command mappings
    generate_command_mappings();
}

fn generate_command_mappings() {
    let commands_dir = Path::new("src/commands");
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("commands_generated.rs");

    let mut command_files = Vec::new();
    
    // Read all .rs files in commands directory (except mod.rs)
    if let Ok(entries) = fs::read_dir(commands_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(file_name) = path.file_name() {
                    if let Some(file_name_str) = file_name.to_str() {
                        if file_name_str.ends_with(".rs") && file_name_str != "mod.rs" {
                            let command_name = file_name_str.strip_suffix(".rs").unwrap();
                            command_files.push(command_name.to_string());
                        }
                    }
                }
            }
        }
    }

    // Generate the command factory code
    let mut generated_code = String::new();
    generated_code.push_str("// Auto-generated command mappings\n\n");

    // Generate the all_commands function
    generated_code.push_str("pub fn all_commands() -> Vec<Box<dyn Command>> {\n");
    generated_code.push_str("    vec![\n");
    
    for command_name in &command_files {
        let struct_name = format!("{}Command", capitalize_first(&command_name));
        generated_code.push_str(&format!(
            "        Box::new({}::{}::default()),\n",
            command_name, struct_name
        ));
    }
    
    generated_code.push_str("    ]\n");
    generated_code.push_str("}\n\n");

    // Generate individual command factory functions
    for command_name in &command_files {
        let struct_name = format!("{}Command", capitalize_first(&command_name));
        let function_name = format!("create_{}_command", command_name);
        generated_code.push_str(&format!(
            "pub fn {}() -> Box<dyn Command> {{\n",
            function_name
        ));
        generated_code.push_str(&format!(
            "    Box::new({}::{}::default())\n",
            command_name, struct_name
        ));
        generated_code.push_str("}\n\n");
    }

    // Write the generated code to the output file
    fs::write(&dest_path, generated_code).unwrap();

    println!("cargo:rerun-if-changed=src/commands");
    for command_name in &command_files {
        println!("cargo:rerun-if-changed=src/commands/{}.rs", command_name);
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}
