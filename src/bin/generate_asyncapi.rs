use serde_json::{json, Value};
use zaakchat::schemas::get_all_schemas;
use std::collections::HashMap;
use std::fs;

fn main() {
    println!("Generating AsyncAPI specification from code...");

    // Get all schemas from the actual schema module
    let schemas = get_all_schemas();
    let schema_names: Vec<String> = schemas.keys().cloned().collect();

    println!(
        "Found {} schemas: {}",
        schema_names.len(),
        schema_names.join(", ")
    );

    // Generate the AsyncAPI specification with URL references
    let asyncapi_spec = generate_asyncapi_spec(&schemas, false);

    // Generate the AsyncAPI specification with embedded schemas for HTML generation
    let asyncapi_spec_embedded = generate_asyncapi_spec(&schemas, true);

    // Write AsyncAPI YAML file (with URL references)
    match serde_yaml::to_string(&asyncapi_spec) {
        Ok(yaml_content) => {
            if let Err(e) = fs::write("asyncapi.yaml", yaml_content) {
                eprintln!("Failed to write asyncapi.yaml: {}", e);
                std::process::exit(1);
            }
            println!("✓ Generated asyncapi.yaml (with URL references)");
        }
        Err(e) => {
            eprintln!("Failed to serialize AsyncAPI to YAML: {}", e);
            std::process::exit(1);
        }
    }

    // Write AsyncAPI JSON file (with URL references)
    match serde_json::to_string_pretty(&asyncapi_spec) {
        Ok(json_content) => {
            if let Err(e) = fs::write("asyncapi.json", json_content) {
                eprintln!("Failed to write asyncapi.json: {}", e);
                std::process::exit(1);
            }
            println!("✓ Generated asyncapi.json (with URL references)");
        }
        Err(e) => {
            eprintln!("Failed to serialize AsyncAPI to JSON: {}", e);
            std::process::exit(1);
        }
    }

    // Write AsyncAPI YAML file for HTML generation (with embedded schemas)
    match serde_yaml::to_string(&asyncapi_spec_embedded) {
        Ok(yaml_content) => {
            if let Err(e) = fs::write("asyncapi-embedded.yaml", yaml_content) {
                eprintln!("Failed to write asyncapi-embedded.yaml: {}", e);
                std::process::exit(1);
            }
            println!(
                "✓ Generated asyncapi-embedded.yaml (with embedded schemas for HTML generation)"
            );
        }
        Err(e) => {
            eprintln!("Failed to serialize embedded AsyncAPI to YAML: {}", e);
            std::process::exit(1);
        }
    }

    println!("✅ AsyncAPI specification generated successfully!");
    println!("📄 Generated files:");
    println!("   - asyncapi.yaml");
    println!("   - asyncapi.json");
    println!("   Total schemas referenced: {}", schemas.len());

    // List all schemas with their URLs
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    println!("📋 Schema references:");
    let mut schema_names: Vec<_> = schemas.keys().collect();
    schema_names.sort();
    for name in schema_names {
        println!("   - {} -> {}/schemas/{}", name, base_url, name);
    }

    println!("\n🚀 View the specification:");
    println!("   AsyncAPI Studio: pnpm run spec-studio");
    println!("   Generate HTML:   pnpm run spec-html");
    println!("\n🔍 Validation:");
    println!("   Structure only:  pnpm run spec-validate");
    println!("   Full validation: Start server first, then validate");
    println!("                    cargo run --bin zaakchat");
    println!("                    # In another terminal:");
    println!("                    pnpm run spec-validate");
}

fn generate_asyncapi_spec(schemas: &HashMap<String, Value>, embed_schemas: bool) -> Value {
    // Base URL from environment or default
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    let server_host = base_url
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let is_https = base_url.starts_with("https://");

    json!({
        "asyncapi": "3.0.0",
        "info": {
            "title": "SSE Delta Snapshot API",
            "version": "1.0.0",
            "description": "Server-Sent Events API for real-time CloudEvents streaming with delta snapshots for Dutch municipal case management",
            "contact": {
                "name": "VNG Realisatie",
                "url": "https://www.vngrealisatie.nl"
            },
            "license": {
                "name": "EUPL-1.2",
                "url": "https://opensource.org/licenses/EUPL-1.2"
            }
        },
        "defaultContentType": "application/json",
        "servers": {
            "development": {
                "host": server_host,
                "protocol": if is_https { "https" } else { "http" },
                "description": "Development/Production server"
            }
        },
        "channels": {
            "/events": {
                "address": "/events",
                "messages": {
                    "CloudEvent": {
                        "$ref": "#/components/messages/CloudEvent"
                    }
                },
                "description": "Server-Sent Events stream for real-time CloudEvents delivery",
                "bindings": {
                    "http": {
                        "type": "request",
                        "method": "GET",
                        "headers": {
                            "type": "object",
                            "properties": {
                                "Accept": {
                                    "type": "string",
                                    "const": "text/event-stream"
                                },
                                "Cache-Control": {
                                    "type": "string",
                                    "const": "no-cache"
                                }
                            }
                        }
                    }
                }
            }
        },
        "operations": {
            "subscribeToEvents": {
                "action": "receive",
                "channel": {
                    "$ref": "#/channels/~1events"
                },
                "title": "Subscribe to CloudEvents Stream",
                "summary": "Receive real-time CloudEvents via Server-Sent Events",
                "description": "Establishes a persistent SSE connection to receive real-time CloudEvents for case management updates including issues, tasks, planning, documents, and comments.",
                "bindings": {
                    "http": {
                        "method": "GET"
                    }
                }
            },
            "sendEvent": {
                "action": "send",
                "channel": {
                    "$ref": "#/channels/~1events"
                },
                "title": "Send CloudEvent",
                "summary": "Submit a CloudEvent to trigger case management actions",
                "description": "Submit a CloudEvent via HTTP POST to create, update, or delete case management entities like issues, tasks, planning items, documents, and comments.",
                "bindings": {
                    "http": {
                        "method": "POST"
                    }
                }
            }
        },
        "components": {
            "messages": {
                "CloudEvent": {
                    "name": "CloudEvent",
                    "title": "CloudEvent 1.0",
                    "summary": "CloudEvents 1.0 compliant event for case management",
                    "description": "A CloudEvent containing case management data following the CloudEvents specification v1.0. The event carries structured data about municipal case management operations.",
                    "contentType": "application/json",
                    "payload": {
                        "$ref": if embed_schemas {
                            "#/components/schemas/CloudEvent".to_string()
                        } else {
                            format!("{}/schemas/CloudEvent", base_url)
                        }
                    },
                    "examples": generate_message_examples(&base_url, embed_schemas)
                }
            },
            "schemas": if embed_schemas {
                generate_embedded_schemas(schemas)
            } else {
                generate_schema_references(&base_url, schemas)
            }
        }
    })
}

fn generate_schema_references(base_url: &str, schemas: &HashMap<String, Value>) -> Value {
    let mut schema_refs = json!({});

    // Create references to the actual hosted schema URLs
    for schema_name in schemas.keys() {
        schema_refs[schema_name] = json!({
            "$ref": format!("{}/schemas/{}", base_url, schema_name)
        });
    }

    schema_refs
}

fn generate_embedded_schemas(schemas: &HashMap<String, Value>) -> Value {
    let mut embedded_schemas = json!({});

    // Embed the actual schemas instead of creating references
    for (schema_name, schema) in schemas {
        embedded_schemas[schema_name] = schema.clone();
    }

    embedded_schemas
}

fn generate_message_examples(base_url: &str, embed_schemas: bool) -> Vec<Value> {
    vec![
        json!({
            "name": "IssueCreated",
            "summary": "New municipal case created",
            "payload": {
                "specversion": "1.0",
                "id": "01HF7K8QZ9X1Y2Z3A4B5C6D7E8",
                "source": "frontend-demo-event",
                "subject": "1",
                "type": "json.commit",
                "time": "2025-01-15T10:30:00Z",
                "datacontenttype": "application/json",
                "dataschema": if embed_schemas {
                    "#/components/schemas/JSONCommit".to_string()
                } else {
                    format!("{}/schemas/JSONCommit", base_url)
                },
                "data": {
                    "schema": format!("{}/schemas/Issue", base_url),
                    "resource_id": "1",
                    "actor": "user@gemeente.nl",
                    "timestamp": "2025-01-15T10:30:00Z",
                    "resource_data": {
                        "id": "1",
                        "title": "Paspoort aanvragen",
                        "description": "Nieuwe paspoort aanvraag ingediend door burger",
                        "status": "open",
                        "assignee": "alice@gemeente.nl",
                        "created_at": "2025-01-15T10:30:00Z"
                    }
                }
            }
        }),
        json!({
            "name": "IssueUpdated",
            "summary": "Municipal case updated with partial changes",
            "payload": {
                "specversion": "1.0",
                "id": "01HF7K8QZ9X1Y2Z3A4B5C6D7E9",
                "source": "frontend-demo-event",
                "subject": "1",
                "type": "json.commit",
                "time": "2025-01-15T11:15:00Z",
                "datacontenttype": "application/json",
                "dataschema": if embed_schemas {
                    "#/components/schemas/JSONCommit".to_string()
                } else {
                    format!("{}/schemas/JSONCommit", base_url)
                },
                "data": {
                    "schema": format!("{}/schemas/Issue", base_url),
                    "resource_id": "1",
                    "actor": "alice@gemeente.nl",
                    "timestamp": "2025-01-15T11:15:00Z",
                    "patch": {
                        "status": "in_progress",
                        "assignee": "bob@gemeente.nl"
                    }
                }
            }
        }),
        json!({
            "name": "IssueDeleted",
            "summary": "Municipal case deleted",
            "payload": {
                "specversion": "1.0",
                "id": "01HF7K8QZ9X1Y2Z3A4B5C6D7F0",
                "source": "frontend-demo-event",
                "subject": "1",
                "type": "json.commit",
                "time": "2025-01-15T16:45:00Z",
                "datacontenttype": "application/json",
                "dataschema": if embed_schemas {
                    "#/components/schemas/JSONCommit".to_string()
                } else {
                    format!("{}/schemas/JSONCommit", base_url)
                },
                "data": {
                    "schema": format!("{}/schemas/Issue", base_url),
                    "resource_id": "1",
                    "actor": "admin@gemeente.nl",
                    "timestamp": "2025-01-15T16:45:00Z",
                    "deleted": true,
                    "deletion_reason": "Duplicate case - merged with case #3"
                }
            }
        }),
        json!({
            "name": "TaskAssigned",
            "summary": "Task assigned to case worker",
            "payload": {
                "specversion": "1.0",
                "id": "01HF7K8QZ9X1Y2Z3A4B5C6D7F1",
                "source": "workflow-engine",
                "subject": "1",
                "type": "json.commit",
                "time": "2025-01-15T10:35:00Z",
                "datacontenttype": "application/json",
                "dataschema": if embed_schemas {
                    "#/components/schemas/JSONCommit".to_string()
                } else {
                    format!("{}/schemas/JSONCommit", base_url)
                },
                "data": {
                    "item_type": "task",
                    "item_id": "task-1001",
                    "actor": "system@gemeente.nl",
                    "timestamp": "2025-01-15T10:35:00Z",
                    "resource_data": {
                        "cta": "Documenten Controleren",
                        "description": "Controleer de ingediende paspoort aanvraag documenten",
                        "url": "/review/passport-1",
                        "completed": false,
                        "deadline": "2025-01-20"
                    },
                    "itemschema": format!("{}/schemas/Task", base_url)
                }
            }
        }),
        json!({
            "name": "TaskCompleted",
            "summary": "Task marked as completed",
            "payload": {
                "specversion": "1.0",
                "id": "01HF7K8QZ9X1Y2Z3A4B5C6D7F2",
                "source": "frontend-user-action",
                "subject": "1",
                "type": "json.commit",
                "time": "2025-01-15T14:30:00Z",
                "datacontenttype": "application/json",
                "dataschema": if embed_schemas {
                    "#/components/schemas/JSONCommit".to_string()
                } else {
                    format!("{}/schemas/JSONCommit", base_url)
                },
                "data": {
                    "item_type": "task",
                    "item_id": "task-1001",
                    "actor": "alice@gemeente.nl",
                    "timestamp": "2025-01-15T14:30:00Z",
                    "patch": {
                        "completed": true,
                        "completed_at": "2025-01-15T14:30:00Z"
                    },
                    "itemschema": format!("{}/schemas/Task", base_url)
                }
            }
        }),
        json!({
            "name": "DocumentUploaded",
            "summary": "Document attached to case",
            "payload": {
                "specversion": "1.0",
                "id": "01HF7K8QZ9X1Y2Z3A4B5C6D7F3",
                "source": "document-service",
                "subject": "1",
                "type": "json.commit",
                "time": "2025-01-15T10:40:00Z",
                "datacontenttype": "application/json",
                "dataschema": if embed_schemas {
                    "#/components/schemas/JSONCommit".to_string()
                } else {
                    format!("{}/schemas/JSONCommit", base_url)
                },
                "data": {
                    "item_type": "document",
                    "item_id": "doc-1001",
                    "actor": "user@gemeente.nl",
                    "timestamp": "2025-01-15T10:40:00Z",
                    "resource_data": {
                        "title": "Paspoortfoto_Officieel.jpg",
                        "url": "https://example.com/documents/passport-photo-12345.jpg",
                        "size": 89765
                    },
                    "itemschema": format!("{}/schemas/Document", base_url)
                }
            }
        }),
        json!({
            "name": "DocumentDeleted",
            "summary": "Document removed from case",
            "payload": {
                "specversion": "1.0",
                "id": "01HF7K8QZ9X1Y2Z3A4B5C6D7F4",
                "source": "document-service",
                "subject": "1",
                "type": "json.commit",
                "time": "2025-01-15T15:20:00Z",
                "datacontenttype": "application/json",
                "dataschema": if embed_schemas {
                    "#/components/schemas/JSONCommit".to_string()
                } else {
                    format!("{}/schemas/JSONCommit", base_url)
                },
                "data": {
                    "schema": format!("{}/schemas/Document", base_url),
                    "resource_id": "doc-1001",
                    "actor": "alice@gemeente.nl",
                    "timestamp": "2025-01-15T15:20:00Z",
                    "deleted": true,
                    "deletion_reason": "Incorrect document uploaded"
                }
            }
        }),
        json!({
            "name": "PlanningCreated",
            "summary": "Case planning timeline created",
            "payload": {
                "specversion": "1.0",
                "id": "01HF7K8QZ9X1Y2Z3A4B5C6D7F5",
                "source": "planning-service",
                "subject": "2",
                "type": "json.commit",
                "time": "2025-01-15T11:00:00Z",
                "datacontenttype": "application/json",
                "dataschema": if embed_schemas {
                    "#/components/schemas/JSONCommit".to_string()
                } else {
                    format!("{}/schemas/JSONCommit", base_url)
                },
                "data": {
                    "item_type": "planning",
                    "item_id": "planning-2001",
                    "actor": "specialist@gemeente.nl",
                    "timestamp": "2025-01-15T11:00:00Z",
                    "resource_data": {
                        "title": "Vergunningsprocedure",
                        "description": "Proces voor het verkrijgen van de benodigde vergunningen",
                        "moments": [
                            {
                                "id": "moment-1",
                                "date": "2025-01-10",
                                "title": "Aanvraag indienen",
                                "status": "completed"
                            },
                            {
                                "id": "moment-2",
                                "date": "2025-01-15",
                                "title": "Behandeling door gemeente",
                                "status": "current"
                            },
                            {
                                "id": "moment-3",
                                "date": "2025-01-25",
                                "title": "Besluit gemeente",
                                "status": "planned"
                            }
                        ]
                    },
                    "itemschema": format!("{}/schemas/Planning", base_url)
                }
            }
        }),
        json!({
            "name": "PlanningUpdated",
            "summary": "Case planning timeline status updated",
            "payload": {
                "specversion": "1.0",
                "id": "01HF7K8QZ9X1Y2Z3A4B5C6D7F6",
                "source": "planning-service",
                "subject": "2",
                "type": "json.commit",
                "time": "2025-01-15T16:00:00Z",
                "datacontenttype": "application/json",
                "dataschema": if embed_schemas {
                    "#/components/schemas/JSONCommit".to_string()
                } else {
                    format!("{}/schemas/JSONCommit", base_url)
                },
                "data": {
                    "item_type": "planning",
                    "item_id": "planning-2001",
                    "actor": "specialist@gemeente.nl",
                    "timestamp": "2025-01-15T16:00:00Z",
                    "patch": {
                        "moments": [
                            {
                                "id": "moment-2",
                                "date": "2025-01-15",
                                "title": "Behandeling door gemeente",
                                "status": "completed"
                            },
                            {
                                "id": "moment-3",
                                "date": "2025-01-25",
                                "title": "Besluit gemeente",
                                "status": "current"
                            }
                        ]
                    },
                    "itemschema": format!("{}/schemas/Planning", base_url)
                }
            }
        }),
        json!({
            "name": "CommentAdded",
            "summary": "Comment added to case",
            "payload": {
                "specversion": "1.0",
                "id": "01HF7K8QZ9X1Y2Z3A4B5C6D7F7",
                "source": "frontend-demo-event",
                "subject": "1",
                "type": "json.commit",
                "time": "2025-01-15T14:20:00Z",
                "datacontenttype": "application/json",
                "dataschema": if embed_schemas {
                    "#/components/schemas/JSONCommit".to_string()
                } else {
                    format!("{}/schemas/JSONCommit", base_url)
                },
                "data": {
                    "item_type": "comment",
                    "item_id": "comment-1001",
                    "actor": "alice@gemeente.nl",
                    "timestamp": "2025-01-15T14:20:00Z",
                    "resource_data": {
                        "content": "Documenten zijn gecontroleerd en goedgekeurd. Zaak kan worden voortgezet.",
                        "parent_id": null,
                        "mentions": []
                    },
                    "itemschema": format!("{}/schemas/Comment", base_url)
                }
            }
        }),
        json!({
            "name": "CommentUpdated",
            "summary": "Comment content edited",
            "payload": {
                "specversion": "1.0",
                "id": "01HF7K8QZ9X1Y2Z3A4B5C6D7F8",
                "source": "frontend-demo-event",
                "subject": "1",
                "type": "json.commit",
                "time": "2025-01-15T14:25:00Z",
                "datacontenttype": "application/json",
                "dataschema": if embed_schemas {
                    "#/components/schemas/JSONCommit".to_string()
                } else {
                    format!("{}/schemas/JSONCommit", base_url)
                },
                "data": {
                    "item_type": "comment",
                    "item_id": "comment-1001",
                    "actor": "alice@gemeente.nl",
                    "timestamp": "2025-01-15T14:25:00Z",
                    "patch": {
                        "content": "Documenten zijn gecontroleerd en goedgekeurd. Zaak kan worden voortgezet. Update: alle vereiste bijlagen zijn aanwezig."
                    },
                    "itemschema": format!("{}/schemas/Comment", base_url)
                }
            }
        }),
        json!({
            "name": "CommentDeleted",
            "summary": "Comment removed from case",
            "payload": {
                "specversion": "1.0",
                "id": "01HF7K8QZ9X1Y2Z3A4B5C6D7F9",
                "source": "frontend-demo-event",
                "subject": "1",
                "type": "json.commit",
                "time": "2025-01-15T17:10:00Z",
                "datacontenttype": "application/json",
                "dataschema": format!("{}/schemas/JSONCommit", base_url),
                "data": {
                    "schema": format!("{}/schemas/Comment", base_url),
                    "resource_id": "comment-1001",
                    "actor": "alice@gemeente.nl",
                    "timestamp": "2025-01-15T17:10:00Z",
                    "deleted": true
                }
            }
        }),
    ]
}
