use axum::{extract::Path, http::StatusCode, Json};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// CloudEvents specification struct
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CloudEvent {
    /// Versie van de CloudEvents specificatie (altijd "1.0")
    pub specversion: String,
    /// Unieke identificatie van deze gebeurtenis
    pub id: String,
    /// Bron systeem dat de gebeurtenis heeft aangemaakt (bijv. "zaaksysteem", "frontend-demo")
    pub source: String,
    /// Het onderwerp van de gebeurtenis, meestal de zaak ID waar het over gaat
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// Type gebeurtenis. Hier is het altijd "json.commit"
    #[serde(rename = "type")]
    pub event_type: String,
    /// Tijdstip waarop de gebeurtenis plaatsvond (ISO 8601 formaat)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    /// Formaat van de data (meestal "application/json")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datacontenttype: Option<String>,
    /// URL naar het schema dat de data beschrijft
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataschema: Option<String>,
    /// Verwijzing naar externe data locatie (indien data niet inline staat)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataref: Option<String>,
    /// Volgnummer voor het ordenen van gebeurtenissen
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<String>,
    /// Type van de volgnummering die gebruikt wordt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequencetype: Option<String>,
    /// De inhoud van de eigenlijke gebeurtenis.
    /// Bij JSONCommits zit hier de daadwerkelijke JSONCommit data in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// JSONCommit - Een commit van wijzigingen aan een JSON resource
///
/// Dit event type vertegenwoordigt elke wijziging aan een JSON resource, of het nu gaat om:
/// - Het aanmaken van een nieuwe resource (resource_data bevat de volledige resource)
/// - Het updaten van een bestaande resource (patch bevat de wijzigingen)
/// - Het verwijderen van een resource (deleted: true markeert de resource als verwijderd)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct JSONCommit {
    /// URL naar het JSON Schema dat de structuur van de resource beschrijft (bijv. "http://localhost:8000/schemas/Comment")
    /// Dit bepaalt welke velden de resource moet hebben en wat hun dataype is.
    pub schema: String,
    /// Unieke identificatie van de resource waar deze commit over gaat.
    pub resource_id: String,
    /// Email van de persoon die de actie heeft uitgevoerd (bijv. "alice@gemeente.nl", "user@gemeente.nl")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    /// Tijdstip waarop de commit plaatsvond (ISO 8601 formaat: 2024-01-15T10:30:00Z)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    /// Complete resource data (bij aanmaken van nieuwe resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource_data: Option<Value>,
    /// JSON Merge Patch (RFC 7396) met wijzigingen (bij updates).
    /// Velden met een null waarde worden verwijderd.
    /// Alle andere velden worden bijgewerkt / overgeschreven.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Value>,
    /// Markeert de resource als verwijderd (bij verwijderingen).
    /// De resource (en de gerelateerde events) moeten dan uit de store verwijderd worden.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
}

/// Soorten items in het zaaksysteem
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    /// Zaak - een burgerzaak of aanvraag die behandeld wordt
    Issue,
    /// Reactie - een opmerking of toelichting bij een zaak
    Comment,
    /// Taak - een actie die uitgevoerd moet worden
    Task,
    /// Planning - een tijdlijn met verschillende momenten/fasen
    Planning,
    /// Document - een bestand of document bij een zaak
    Document,
}

/// Document dat bij een zaak hoort (bijv. paspoortfoto, uittreksel GBA)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Document {
    /// Bestandsnaam of titel van het document (bijv. "Paspoortfoto_Jan_Jansen.jpg")
    pub title: String,
    /// Download URL van het document - moet toegankelijk zijn voor geautoriseerde gebruikers
    pub url: String,
    /// Bestandsgrootte in bytes
    pub size: u64,
}

/// Zaak - een burgerzaak of aanvraag die door de gemeente behandeld wordt
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Issue {
    /// Korte, duidelijke titel van de zaak (bijv. "Paspoort aanvragen", "Kapvergunning Dorpsstraat 12")
    pub title: String,
    /// Uitgebreide beschrijving: wat is de aanvraag, welke stappen zijn al ondernomen
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Huidige behandelstatus van de zaak
    pub status: IssueStatus,
    /// Email van de ambtenaar die de zaak behandelt (bijv. "alice@gemeente.nl")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    /// Lijst van betrokken personen (emails) bij deze zaak
    #[serde(skip_serializing_if = "Option::is_none")]
    pub involved: Option<Vec<String>>,
}

/// Taak - een actie die uitgevoerd moet worden om een zaak te behandelen
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Task {
    /// Korte actie-omschrijving (bijv. "Documenten controleren", "Afspraak inplannen")
    pub cta: String,
    /// Uitgebreide uitleg: wat moet er precies gebeuren, welke voorwaarden gelden
    pub description: String,
    /// Link naar de plaats waar de taak uitgevoerd kan worden (bijv. formulier, overzicht)
    pub url: String,
    /// Is de taak voltooid? (true = klaar, false = nog te doen)
    pub completed: bool,
    /// Uiterste datum voor voltooiing (YYYY-MM-DD, bijv. "2024-01-25")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deadline: Option<String>,
}

/// Status van een zaak in behandeling
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IssueStatus {
    /// Nieuw binnengekomen, nog niet in behandeling genomen
    Open,
    /// Wordt momenteel behandeld door een ambtenaar
    #[serde(rename = "in_progress")]
    InProgress,
    /// Behandeling afgerond, zaak is gesloten
    Closed,
}

/// Reactie - een opmerking, vraag of toelichting bij een zaak
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Comment {
    /// De tekst van de reactie (bijv. "Documenten zijn goedgekeurd", "Burger gebeld voor aanvullende info")
    pub content: String,
    /// ID van de reactie waar dit een antwoord op is (voor discussies met meerdere berichten)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// Email adressen van collega's die specifiek genoemd worden (bijv. "@alice@gemeente.nl")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mentions: Option<Vec<String>>,
}

/// Planning - een tijdlijn met verschillende stappen of fasen voor zaakbehandeling
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Planning {
    /// Naam van de planning (bijv. "Vergunningsprocedure", "Paspoort aanvraag proces")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Uitleg over wat deze planning behelst en welke stappen doorlopen worden
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Alle stappen/momenten in deze planning, in chronologische volgorde
    pub moments: Vec<PlanningMoment>,
}

/// Een specifieke stap of mijlpaal binnen een planning
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlanningMoment {
    /// Geplande of gerealiseerde datum (YYYY-MM-DD, bijv. "2024-01-15")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    /// Naam van deze stap (bijv. "Intake gesprek", "Documentcheck", "Besluit gemeente")
    pub title: String,
    /// In welke fase dit moment zich bevindt
    pub status: PlanningStatus,
}

/// Status van een planning moment
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlanningStatus {
    /// Afgerond - deze stap is voltooid
    Completed,
    /// Huidig - deze stap wordt nu uitgevoerd
    Current,
    /// Gepland - deze stap staat nog in de toekomst
    Planned,
}

/// Resolve schema references recursively
fn resolve_schema_refs(mut schema: Value, all_schemas: &HashMap<String, Value>) -> Value {
    fn resolve_refs_recursive(value: &mut Value, schemas: &HashMap<String, Value>) {
        match value {
            Value::Object(map) => {
                // Handle direct $ref
                if let Some(ref_value) = map.get("$ref") {
                    if let Some(ref_str) = ref_value.as_str() {
                        if let Some(definition_name) = ref_str.strip_prefix("#/definitions/") {
                            if let Some(definition) = schemas.get(definition_name) {
                                *value = definition.clone();
                                resolve_refs_recursive(value, schemas);
                                return;
                            }
                        }
                    }
                }

                // Handle allOf with $ref patterns
                if let Some(Value::Array(all_of_array)) = map.get_mut("allOf") {
                    if all_of_array.len() == 1 {
                        if let Some(Value::Object(ref_obj)) = all_of_array.get(0) {
                            if let Some(ref_value) = ref_obj.get("$ref") {
                                if let Some(ref_str) = ref_value.as_str() {
                                    if let Some(definition_name) =
                                        ref_str.strip_prefix("#/definitions/")
                                    {
                                        if let Some(definition) = schemas.get(definition_name) {
                                            // Replace the allOf with the resolved definition
                                            map.remove("allOf");
                                            if let Value::Object(def_map) = definition {
                                                for (key, val) in def_map.iter() {
                                                    if !map.contains_key(key) {
                                                        map.insert(key.clone(), val.clone());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                for (_, v) in map.iter_mut() {
                    resolve_refs_recursive(v, schemas);
                }
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    resolve_refs_recursive(item, schemas);
                }
            }
            _ => {}
        }
    }
    resolve_refs_recursive(&mut schema, all_schemas);
    schema
}

/// Extract definitions from schema
fn extract_definitions(
    schema: &schemars::schema::RootSchema,
    schemas: &mut HashMap<String, Value>,
) {
    for (name, definition) in &schema.definitions {
        schemas.insert(name.clone(), serde_json::to_value(definition).unwrap());
    }
}

/// Macro to generate schemas for multiple types
macro_rules! generate_schemas {
    ($($type_name:ident),+ $(,)?) => {
        {
            // Generate all schemas
            $(
                let _schema = schema_for!($type_name);
            )+

            // Collect all definitions
            let mut all_definitions = HashMap::new();
            $(
                let schema = schema_for!($type_name);
                extract_definitions(&schema, &mut all_definitions);
            )+

            // Generate resolved schemas
            let mut schemas = HashMap::new();
            $(
                let schema = schema_for!($type_name);
                let mut schema_json = serde_json::to_value(&schema).unwrap();
                schema_json = resolve_schema_refs(schema_json, &all_definitions);
                schemas.insert(stringify!($type_name).to_string(), schema_json);
            )+

            // Add resolved definitions
            let definitions_for_resolving = all_definitions.clone();
            for (name, definition) in all_definitions {
                let resolved = resolve_schema_refs(definition, &definitions_for_resolving);
                schemas.insert(name, resolved);
            }

            schemas
        }
    };
}

/// Get all JSON schemas as a HashMap
pub fn get_all_schemas() -> HashMap<String, Value> {
    generate_schemas![
        CloudEvent,
        JSONCommit,
        ItemType,
        Document,
        Issue,
        IssueStatus,
        Task,
        Comment,
        Planning,
        PlanningMoment,
        PlanningStatus
    ]
}

/// Get all available schemas as an index
pub async fn handle_get_schemas_index() -> Json<Value> {
    Json(get_schema_index())
}

/// Get a specific schema by name
pub async fn handle_get_schema(Path(name): Path<String>) -> Result<Json<Value>, StatusCode> {
    match get_schema(&name) {
        Some(schema) => Ok(Json(schema)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Get a specific schema by name
pub fn get_schema(name: &str) -> Option<Value> {
    let schemas = get_all_schemas();
    schemas.get(name).cloned()
}

/// Get schema index (list of all available schemas)
pub fn get_schema_index() -> Value {
    let schemas = get_all_schemas();
    let schema_names: Vec<String> = schemas.keys().cloned().collect();

    json!({
        "schemas": schema_names,
        "base_url": "/schemas",
        "description": "Available JSON schemas for CloudEvents and data types"
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_schema_index() {
        let index = get_schema_index();

        assert!(index.is_object());
        assert!(index.get("schemas").is_some());
        assert!(index.get("base_url").is_some());
        assert!(index.get("description").is_some());

        let schemas = index.get("schemas").unwrap().as_array().unwrap();
        assert!(schemas.len() > 0);

        // Check that key schemas are present
        let schema_names: Vec<String> = schemas
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();

        assert!(schema_names.contains(&"CloudEvent".to_string()));
        assert!(schema_names.contains(&"Issue".to_string()));
        assert!(schema_names.contains(&"Task".to_string()));
        assert!(schema_names.contains(&"Planning".to_string()));
        // Test that previously missing schemas are now included
        assert!(schema_names.contains(&"Document".to_string()));
        assert!(schema_names.contains(&"ItemType".to_string()));
        assert!(schema_names.contains(&"IssueStatus".to_string()));
        assert!(schema_names.contains(&"PlanningStatus".to_string()));
    }

    #[test]
    fn test_get_specific_schema() {
        // Test CloudEvent schema
        let cloud_event_schema = get_schema("CloudEvent");
        assert!(cloud_event_schema.is_some());

        let schema = cloud_event_schema.unwrap();
        assert!(schema.get("properties").is_some());

        let properties = schema.get("properties").unwrap().as_object().unwrap();
        assert!(properties.contains_key("specversion"));
        assert!(properties.contains_key("id"));
        assert!(properties.contains_key("source"));
        assert!(properties.contains_key("type"));
        assert!(properties.contains_key("dataschema"));
        assert!(properties.contains_key("dataref"));
        assert!(properties.contains_key("sequence"));
        assert!(properties.contains_key("sequencetype"));
    }

    #[test]
    fn test_get_nonexistent_schema() {
        let result = get_schema("NonExistentSchema");
        assert!(result.is_none());
    }

    #[test]
    fn test_cloud_event_has_nl_gov_fields() {
        let schema = get_schema("CloudEvent").unwrap();
        let properties = schema.get("properties").unwrap().as_object().unwrap();

        // Verify NL-GOV CloudEvents compliance fields are present
        assert!(
            properties.contains_key("dataschema"),
            "Missing dataschema field for NL-GOV compliance"
        );
        assert!(
            properties.contains_key("dataref"),
            "Missing dataref field for NL-GOV compliance"
        );
        assert!(
            properties.contains_key("sequence"),
            "Missing sequence field for NL-GOV compliance"
        );
        assert!(
            properties.contains_key("sequencetype"),
            "Missing sequencetype field for NL-GOV compliance"
        );
    }

    #[test]
    fn test_issue_schema_structure() {
        let schema = get_schema("Issue").unwrap();
        let properties = schema.get("properties").unwrap().as_object().unwrap();

        // Verify key Issue fields
        assert!(properties.contains_key("title"));
        assert!(properties.contains_key("status"));
    }

    #[test]
    fn test_all_schemas_are_valid_json() {
        let all_schemas = get_all_schemas();

        for (name, schema) in all_schemas {
            // Verify each schema is valid JSON and has expected structure
            assert!(
                schema.is_object(),
                "Schema {} is not a valid JSON object",
                name
            );

            // Most schemas should have properties (except enums)
            if !name.ends_with("Status") && !name.ends_with("Type") {
                assert!(
                    schema.get("properties").is_some(),
                    "Schema {} missing properties field",
                    name
                );
            }
        }
    }

    #[test]
    fn test_missing_schemas_now_included() {
        let all_schemas = get_all_schemas();

        // Verify that previously missing schemas are now included
        assert!(
            all_schemas.contains_key("Document"),
            "Document schema missing"
        );
        assert!(
            all_schemas.contains_key("ItemType"),
            "ItemType schema missing"
        );
        assert!(
            all_schemas.contains_key("IssueStatus"),
            "IssueStatus schema missing"
        );
        assert!(
            all_schemas.contains_key("PlanningStatus"),
            "PlanningStatus schema missing"
        );

        // Verify all main types are present
        let expected_schemas = vec![
            "CloudEvent",
            "JSONCommit",
            "ItemType",
            "Document",
            "Issue",
            "IssueStatus",
            "Task",
            "Comment",
            "Planning",
            "PlanningMoment",
            "PlanningStatus",
        ];

        for schema_name in expected_schemas {
            assert!(
                all_schemas.contains_key(schema_name),
                "Missing schema: {}",
                schema_name
            );
        }
    }

    #[test]
    fn test_schema_generation_completeness() {
        let schemas = get_all_schemas();

        // Print schema count for debugging
        println!("Total schemas generated: {}", schemas.len());

        // Verify we have at least the expected number of main schemas
        assert!(
            schemas.len() >= 11,
            "Expected at least 11 schemas, got {}",
            schemas.len()
        );

        // Test that we can get a specific schema
        let cloud_event = get_schema("CloudEvent");
        assert!(
            cloud_event.is_some(),
            "CloudEvent schema should be available"
        );

        let document = get_schema("Document");
        assert!(document.is_some(), "Document schema should be available");

        let item_type = get_schema("ItemType");
        assert!(
            item_type.is_some(),
            "ItemType enum schema should be available"
        );
    }
}

#[tokio::test]
async fn test_get_specific_schema_endpoint() {
    use axum::extract::Path;
    // Call handler and unwrap Json wrapper
    let path = Path("CloudEvent".to_string());
    let json = handle_get_schema(path)
        .await
        .expect("CloudEvent schema should exist");
    let schema = json.0;

    assert!(schema.is_object());
    assert!(schema.get("properties").is_some());

    let properties = schema.get("properties").unwrap().as_object().unwrap();
    assert!(properties.contains_key("specversion"));
    assert!(properties.contains_key("id"));
    assert!(properties.contains_key("source"));
    assert!(properties.contains_key("dataschema"));
    assert!(properties.contains_key("dataref"));
}

#[tokio::test]
async fn test_get_nonexistent_schema_endpoint() {
    use axum::extract::Path;
    use axum::http::StatusCode;

    // Test getting non-existent schema
    let path = Path("NonExistentSchema".to_string());
    let result = handle_get_schema(path).await;

    assert!(result.is_err());
    let status = result.unwrap_err();
    assert_eq!(status, StatusCode::NOT_FOUND);
}
