use zaakchat::schemas::get_all_schemas;
use std::fs;
use std::path::Path;

fn main() {
    println!("Exporting JSON schemas...");

    // Get all schemas
    let schemas = get_all_schemas();

    // Create output directory
    let output_dir = Path::new("target/schemas");
    if let Err(e) = fs::create_dir_all(output_dir) {
        eprintln!("Failed to create output directory: {}", e);
        std::process::exit(1);
    }

    // Write each schema to a separate file
    for (name, schema) in &schemas {
        let filename = format!("{}.json", name);
        let filepath = output_dir.join(filename);

        match serde_json::to_string_pretty(schema) {
            Ok(json_content) => {
                if let Err(e) = fs::write(&filepath, json_content) {
                    eprintln!("Failed to write schema {}: {}", name, e);
                    std::process::exit(1);
                }
                println!("✓ Exported schema: {}", name);
            }
            Err(e) => {
                eprintln!("Failed to serialize schema {}: {}", name, e);
                std::process::exit(1);
            }
        }
    }

    // Also write a combined schemas file
    let combined_path = output_dir.join("all_schemas.json");
    match serde_json::to_string_pretty(&schemas) {
        Ok(json_content) => {
            if let Err(e) = fs::write(&combined_path, json_content) {
                eprintln!("Failed to write combined schemas: {}", e);
                std::process::exit(1);
            }
            println!("✓ Exported combined schemas file");
        }
        Err(e) => {
            eprintln!("Failed to serialize combined schemas: {}", e);
            std::process::exit(1);
        }
    }

    // Write schema index
    let schema_names: Vec<String> = schemas.keys().cloned().collect();
    let index = serde_json::json!({
        "schemas": schema_names,
        "base_url": "/schemas",
        "description": "Available JSON schemas for CloudEvents and data types"
    });

    let index_path = output_dir.join("index.json");
    match serde_json::to_string_pretty(&index) {
        Ok(json_content) => {
            if let Err(e) = fs::write(&index_path, json_content) {
                eprintln!("Failed to write schema index: {}", e);
                std::process::exit(1);
            }
            println!("✓ Exported schema index");
        }
        Err(e) => {
            eprintln!("Failed to serialize schema index: {}", e);
            std::process::exit(1);
        }
    }

    println!("✅ All schemas exported to target/schemas/");
    println!("   Total schemas: {}", schemas.len());
}
