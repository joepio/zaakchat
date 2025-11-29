//! Issue management and CloudEvent generation for the demo application.
//!
//! This module provides functionality to:
//! - Generate initial demo issues with various statuses
//! - Create CloudEvents for different issue operations (create, update, delete)
//! - Generate timeline events (comments, tasks, planning, etc.)
//! - Apply JSON Merge Patch operations to issue data

use crate::schemas::CloudEvent;
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

// Constants for event generation
const DEFAULT_SOURCE: &str = "server-demo-event";
const HOURS_BACK_FOR_INITIAL_DATA: i64 = 3;

// Event type constants
const EVENT_TYPE_JSON_COMMIT: &str = "json.commit";

// Content type constants
const CONTENT_TYPE_JSON: &str = "application/json";

// Event schema constants
const JSON_COMMIT_SCHEMA: &str = "http://localhost:8000/schemas/JSONCommit";
const ISSUE_SCHEMA: &str = "http://localhost:8000/schemas/Issue";
const COMMENT_SCHEMA: &str = "http://localhost:8000/schemas/Comment";
const TASK_SCHEMA: &str = "http://localhost:8000/schemas/Task";
const PLANNING_SCHEMA: &str = "http://localhost:8000/schemas/Planning";
const LLM_ANALYSIS_SCHEMA: &str = "http://localhost:8000/schemas/LLMAnalysis";
const STATUS_CHANGE_SCHEMA: &str = "http://localhost:8000/schemas/StatusChange";
const DOCUMENT_SCHEMA: &str = "http://localhost:8000/schemas/Document";

// Issue templates for initial data generation
const ISSUE_TEMPLATES: &[(&str, &str, Option<&str>)] = &[
    (
        "Nieuw paspoort aanvragen",
        "Burger wil nieuw paspoort aanvragen",
        Some("alice@gemeente.nl"),
    ),
    (
        "Melding overlast",
        "Geluidsoverlast buren gemeld",
        Some("bob@gemeente.nl"),
    ),
    (
        "Verhuizing doorgeven",
        "Adreswijziging registreren in BRP",
        None,
    ),
    (
        "Parkeervergunning aanvraag",
        "Bewoner wil parkeervergunning voor nieuwe auto",
        Some("carol@gemeente.nl"),
    ),
    (
        "Kapvergunning boom",
        "Vergunning nodig voor kappen boom in achtertuin",
        Some("dave@gemeente.nl"),
    ),
    (
        "Uitkering aanvragen",
        "Burger vraagt bijstandsuitkering aan",
        None,
    ),
    (
        "Klacht over dienstverlening",
        "Ontevreden over behandeling bij balie",
        Some("eve@gemeente.nl"),
    ),
    (
        "Huwelijk voltrekken",
        "Koppel wil trouwen op gemeentehuis",
        Some("frank@gemeente.nl"),
    ),
    (
        "WOZ-bezwaar indienen",
        "Eigenaar niet eens met WOZ-waardering",
        None,
    ),
    (
        "Hondenbelasting",
        "Registratie nieuwe hond voor hondenbelasting",
        Some("grace@gemeente.nl"),
    ),
];

// Patch operations for initial data
fn get_patch_operations() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        (
            "1",
            json!({"status": "in_progress", "assignee": "alice@gemeente.nl"}),
        ),
        (
            "3",
            json!({"status": "in_progress", "assignee": "bob@gemeente.nl"}),
        ),
        ("5", json!({"status": "closed", "resolution": "fixed"})),
        ("4", json!({"assignee": null, "status": "open"})),
        ("7", json!({"status": "in_progress"})),
        ("8", json!({"status": "closed", "resolution": "completed"})),
    ]
}

// Delete operations for initial data
const DELETE_OPERATIONS: &[(&str, &str)] = &[("9", "duplicate"), ("10", "invalid request")];

/// Represents a timeline operation for generating demo data
#[derive(Debug, Clone)]
struct TimelineOperation {
    issue_id: &'static str,
    item_type: &'static str,
    item_id: &'static str,
    actor: &'static str,
    item_data: serde_json::Value,
    minute_offset: i64,
}

impl TimelineOperation {
    fn new(
        issue_id: &'static str,
        item_type: &'static str,
        item_id: &'static str,
        actor: &'static str,
        item_data: serde_json::Value,
        minute_offset: i64,
    ) -> Self {
        Self {
            issue_id,
            item_type,
            item_id,
            actor,
            item_data,
            minute_offset,
        }
    }
}

/// Generate initial issues and their CloudEvents
pub fn generate_initial_data() -> (Vec<Value>, HashMap<String, Value>) {
    let mut events = Vec::new();
    let mut issues = HashMap::new();
    let base_time = Utc::now() - Duration::hours(HOURS_BACK_FOR_INITIAL_DATA);

    // Generate initial issues
    generate_initial_issues(&mut events, &mut issues, base_time);

    // Add patch events
    add_patch_events(&mut events, &mut issues, base_time);

    // Add delete events
    add_delete_events(&mut events, &mut issues, base_time);

    // Add timeline events
    add_timeline_events(&mut events, base_time);

    // Add planning events
    add_planning_events(&mut events, base_time);

    // Add final update event
    add_timeline_update_event(&mut events, base_time);

    (events, issues)
}

/// Generate create events for initial issues
fn generate_initial_issues(
    events: &mut Vec<Value>,
    issues: &mut HashMap<String, Value>,
    base_time: chrono::DateTime<chrono::Utc>,
) {
    for (i, (title, description, assignee)) in ISSUE_TEMPLATES.iter().enumerate() {
        let issue_id = (i + 1).to_string();
        let mut create_event =
            generate_create_event_with_data(&issue_id, title, description, *assignee);

        // Set historical timestamp
        let create_time = base_time + Duration::minutes(i as i64 * 2);
        create_event["time"] = json!(create_time.to_rfc3339());

        // Extract issue data and add to issues state
        if let Some(data) = create_event.get("data") {
            if let Some(resource_data) = data.get("resource_data").cloned() {
                issues.insert(issue_id.clone(), resource_data);
            }
        }

        events.push(create_event);
    }
}

/// Add patch events to modify existing issues
fn add_patch_events(
    events: &mut Vec<Value>,
    issues: &mut HashMap<String, Value>,
    base_time: chrono::DateTime<chrono::Utc>,
) {
    let patch_operations = get_patch_operations();
    for (i, (issue_id, patch_data)) in patch_operations.iter().enumerate() {
        let mut patch_event = generate_patch_event_with_data(issue_id, patch_data);

        // Set historical timestamp
        let patch_time = base_time + Duration::minutes(30 + (i as i64 * 3));
        patch_event["time"] = json!(patch_time.to_rfc3339());

        // Apply patch to issues state
        if let Some(existing_issue) = issues.get_mut(&issue_id.to_string()) {
            if let Some(data) = patch_event.get("data") {
                if let Some(patch) = data.get("patch") {
                    apply_merge_patch(existing_issue, patch);
                }
            }
        }

        events.push(patch_event);
    }
}

/// Add delete events for some issues
fn add_delete_events(
    events: &mut Vec<Value>,
    issues: &mut HashMap<String, Value>,
    base_time: chrono::DateTime<chrono::Utc>,
) {
    for (i, (issue_id, reason)) in DELETE_OPERATIONS.iter().enumerate() {
        let mut delete_event = generate_delete_event_with_data(issue_id, reason);

        // Set historical timestamp
        let delete_time = base_time + Duration::minutes(60 + (i as i64 * 5));
        delete_event["time"] = json!(delete_time.to_rfc3339());

        events.push(delete_event);
        issues.remove(&issue_id.to_string());
    }
}

/// Add timeline events (comments, tasks, etc.)
fn add_timeline_events(events: &mut Vec<Value>, base_time: chrono::DateTime<chrono::Utc>) {
    let timeline_operations = get_timeline_operations();
    for operation in timeline_operations.iter() {
        let schema = get_schema_for_item_type(operation.item_type);
        let mut timeline_event = create_cloud_event(
            DEFAULT_SOURCE,
            Some(operation.issue_id),
            CONTENT_TYPE_JSON,
            operation.item_id,
            Some(&operation.item_data),
            None,
            schema,
        );

        // Override time and add timeline-specific data
        let event_time = (base_time + Duration::minutes(operation.minute_offset)).to_rfc3339();
        timeline_event["time"] = json!(event_time);
        if let Some(data) = timeline_event.get_mut("data") {
            data["actor"] = json!(operation.actor);
            data["timestamp"] = json!(event_time);
        }

        events.push(timeline_event);
    }
}

/// Add planning events
fn add_planning_events(events: &mut Vec<Value>, base_time: chrono::DateTime<chrono::Utc>) {
    let planning_operations = get_planning_operations();
    for operation in planning_operations.iter() {
        let mut planning_event = create_cloud_event(
            DEFAULT_SOURCE,
            Some(operation.issue_id),
            CONTENT_TYPE_JSON,
            operation.item_id,
            Some(&operation.item_data),
            None,
            PLANNING_SCHEMA,
        );

        // Override time and add timeline-specific data
        let event_time = (base_time + Duration::minutes(operation.minute_offset)).to_rfc3339();
        planning_event["time"] = json!(event_time);
        if let Some(data) = planning_event.get_mut("data") {
            data["actor"] = json!(operation.actor);
            data["timestamp"] = json!(event_time);
        }

        events.push(planning_event);
    }
}

/// Add a final timeline update event
fn add_timeline_update_event(events: &mut Vec<Value>, base_time: chrono::DateTime<chrono::Utc>) {
    let patch_data = json!({
        "content": "Update: Formulier is nu compleet ingevuld."
    });

    let mut timeline_update_event = create_cloud_event(
        DEFAULT_SOURCE,
        Some("1"),
        CONTENT_TYPE_JSON,
        "comment-1001",
        None,
        Some(&patch_data),
        COMMENT_SCHEMA,
    );

    // Override time and add timeline-specific data
    let event_time = (base_time + Duration::minutes(90)).to_rfc3339();
    timeline_update_event["time"] = json!(event_time);
    if let Some(data) = timeline_update_event.get_mut("data") {
        data["actor"] = json!("alice@example.com");
        data["timestamp"] = json!(event_time);
    }

    events.push(timeline_update_event);
}

/// Get the appropriate schema for an item type
fn get_schema_for_item_type(item_type: &str) -> &'static str {
    match item_type {
        "task" => TASK_SCHEMA,
        "comment" => COMMENT_SCHEMA,
        "llm_analysis" => LLM_ANALYSIS_SCHEMA,
        "status_change" => STATUS_CHANGE_SCHEMA,
        "planning" => PLANNING_SCHEMA,
        "document" => DOCUMENT_SCHEMA,
        _ => JSON_COMMIT_SCHEMA,
    }
}

/// Get predefined timeline operations for initial data
fn get_timeline_operations() -> Vec<TimelineOperation> {
    vec![
        TimelineOperation::new(
            "1",
            "comment",
            "comment-1001",
            "alice@gemeente.nl",
            json!({
                "content": "Ik ben deze zaak aan het behandelen. Meer informatie volgt.",
                "parent_id": null,
                "mentions": ["@bob"]
            }),
            105,
        ),
        TimelineOperation::new(
            "2",
            "status_change",
            "status-1002",
            "bob@gemeente.nl",
            json!({
                "field": "status",
                "old_value": "open",
                "new_value": "in_progress",
                "reason": "Start onderzoek"
            }),
            110,
        ),
        TimelineOperation::new(
            "1",
            "llm_analysis",
            "llm-1003",
            "system@example.com",
            json!({
                "prompt": "Analyze this authentication issue and provide recommendations",
                "response": "This appears to be related to session timeout configuration. The authentication system is likely expiring sessions too quickly, causing users to be logged out unexpectedly.",
                "model": "gpt-4",
                "confidence": 0.87
            }),
            115,
        ),
        TimelineOperation::new(
            "2",
            "comment",
            "comment-1004",
            "alice@gemeente.nl",
            json!({
                "content": "De zaak is in behandeling genomen en doorgestuurd naar de juiste afdeling.",
                "parent_id": null,
                "mentions": []
            }),
            120,
        ),
        TimelineOperation::new(
            "1",
            "task",
            "task-1005",
            "system@gemeente.nl",
            json!({
                "cta": "Documenten Controleren",
                "description": "Controleer de ingediende paspoort aanvraag documenten",
                "url": "/review/passport-1",
                "completed": false,
                "deadline": "2025-09-26"
            }),
            125,
        ),
        TimelineOperation::new(
            "2",
            "task",
            "task-1006",
            "workflow@gemeente.nl",
            json!({
                "cta": "Locatie Inspecteren",
                "description": "Voer inspectie ter plaatse uit voor geluidsoverlast melding",
                "url": "/inspect/noise-complaint-2",
                "completed": false,
                "deadline": "2025-09-24"
            }),
            130,
        ),
        TimelineOperation::new(
            "3",
            "task",
            "task-1007",
            "system@gemeente.nl",
            json!({
                "cta": "Aanvrager Bellen",
                "description": "Bel aanvrager om nieuwe adresgegevens te bevestigen",
                "url": "/contact/applicant-3",
                "completed": false,
                "deadline": "2025-09-25"
            }),
            135,
        ),
        TimelineOperation::new(
            "1",
            "document",
            "doc-1008",
            "alice@gemeente.nl",
            json!({
                "title": "Paspoort_Aanvraag_Formulier.pdf",
                "url": "https://example.com/documents/paspoort-aanvraag-12345.pdf",
                "size": 152985
            }),
            140,
        ),
        TimelineOperation::new(
            "2",
            "document",
            "doc-1009",
            "system@gemeente.nl",
            json!({
                "title": "Geluidsmeting_Rapport_September.docx",
                "url": "https://example.com/documents/geluidsmeting-sept-2024.docx",
                "size": 2847563
            }),
            145,
        ),
        TimelineOperation::new(
            "3",
            "document",
            "doc-1010",
            "bob@gemeente.nl",
            json!({
                "title": "Adreswijziging_Bevestiging.pdf",
                "url": "https://example.com/documents/adreswijziging-bevestiging.pdf",
                "size": 89472
            }),
            150,
        ),
    ]
}
/// Get predefined planning operations for initial data
fn get_planning_operations() -> Vec<TimelineOperation> {
    vec![
        TimelineOperation::new(
            "1",
            "planning",
            "planning-1001",
            "specialist@gemeente.nl",
            json!({
                "title": "Paspoort procedure",
                "description": "Stappen voor verwerking paspoort aanvraag",
                "moments": [
                    {
                        "id": "moment-1001-1",
                        "date": "2024-12-18",
                        "title": "Aanvraag ontvangen",
                        "status": "completed"
                    },
                    {
                        "id": "moment-1001-2",
                        "date": "2024-12-19",
                        "title": "Documenten controleren",
                        "status": "current"
                    },
                    {
                        "id": "moment-1001-3",
                        "date": "2024-12-23",
                        "title": "Foto en vingerafdrukken",
                        "status": "planned"
                    },
                    {
                        "id": "moment-1001-4",
                        "date": "2024-12-30",
                        "title": "Paspoort uitreiken",
                        "status": "planned"
                    }
                ]
            }),
            140,
        ),
        TimelineOperation::new(
            "2",
            "planning",
            "planning-1002",
            "bob@gemeente.nl",
            json!({
                "title": "Overlast onderzoek",
                "description": "Plan voor onderzoek geluidsoverlast",
                "moments": [
                    {
                        "id": "moment-1002-1",
                        "date": "2024-12-18",
                        "title": "Melding geregistreerd",
                        "status": "completed"
                    },
                    {
                        "id": "moment-1002-2",
                        "date": "2024-12-20",
                        "title": "Locatie inspectie",
                        "status": "current"
                    },
                    {
                        "id": "moment-1002-3",
                        "date": "2024-12-27",
                        "title": "Rapport opstellen",
                        "status": "planned"
                    },
                    {
                        "id": "moment-1002-4",
                        "date": "2025-01-03",
                        "title": "Besluit communiceren",
                        "status": "planned"
                    }
                ]
            }),
            145,
        ),
        TimelineOperation::new(
            "3",
            "planning",
            "planning-1003",
            "system@gemeente.nl",
            json!({
                "title": "Adreswijziging verwerking",
                "description": "Stappen voor verwerken verhuizing",
                "moments": [
                    {
                        "id": "moment-1003-1",
                        "date": "2024-12-18",
                        "title": "Verhuizing gemeld",
                        "status": "completed"
                    },
                    {
                        "id": "moment-1003-2",
                        "date": "2024-12-21",
                        "title": "Gegevens verifiëren",
                        "status": "current"
                    },
                    {
                        "id": "moment-1003-3",
                        "date": "2024-12-28",
                        "title": "BRP bijwerken",
                        "status": "planned"
                    }
                ]
            }),
            150,
        ),
        TimelineOperation::new(
            "4",
            "planning",
            "planning-1004",
            "carol@gemeente.nl",
            json!({
                "title": "Parkeervergunning proces",
                "description": "Verwerking parkeervergunning aanvraag",
                "moments": [
                    {
                        "id": "moment-1004-1",
                        "date": "2024-12-18",
                        "title": "Aanvraag ontvangen",
                        "status": "completed"
                    },
                    {
                        "id": "moment-1004-2",
                        "date": "2024-12-22",
                        "title": "Locatie controleren",
                        "status": "planned"
                    },
                    {
                        "id": "moment-1004-3",
                        "date": "2024-12-29",
                        "title": "Vergunning uitgeven",
                        "status": "planned"
                    }
                ]
            }),
            155,
        ),
    ]
}
/// Apply JSON Merge Patch (RFC 7396) to a target JSON value
pub fn apply_merge_patch(target: &mut Value, patch: &Value) {
    if let (Value::Object(target_obj), Value::Object(patch_obj)) = (target, patch) {
        for (key, patch_value) in patch_obj {
            match patch_value {
                Value::Null => {
                    // Remove the field
                    target_obj.remove(key);
                }
                _ => {
                    // Set or replace the field
                    if let Some(target_value) = target_obj.get_mut(key) {
                        if target_value.is_object() && patch_value.is_object() {
                            // Recursively merge objects
                            apply_merge_patch(target_value, patch_value);
                        } else {
                            // Replace the value
                            *target_value = patch_value.clone();
                        }
                    } else {
                        // Add new field
                        target_obj.insert(key.clone(), patch_value.clone());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_initial_data() {
        let (events, issues) = generate_initial_data();

        // Should have events for creation, patches, and deletes
        assert!(events.len() > 10);

        // Should have 8 issues remaining (10 created, 2 deleted)
        assert_eq!(issues.len(), 8);

        // All events should be valid CloudEvents
        for event in &events {
            assert!(event.get("specversion").is_some());
            assert!(event.get("id").is_some());
            assert!(event.get("source").is_some());
            assert!(event.get("type").is_some());
        }
    }

    #[test]
    fn test_apply_merge_patch() {
        let mut target = json!({
            "title": "Originele Zaak",
            "status": "open",
            "assignee": "john@gemeente.nl"
        });

        let patch = json!({
            "status": "closed",
            "assignee": null,
            "resolution": "fixed"
        });

        apply_merge_patch(&mut target, &patch);

        assert_eq!(target["status"], "closed");
        assert_eq!(target["resolution"], "fixed");
        assert_eq!(target["assignee"], Value::Null);
        assert_eq!(target["title"], "Originele Zaak"); // unchanged
    }

    #[test]
    fn test_generate_demo_event() {
        let mut issues = HashMap::new();
        issues.insert(
            "1".to_string(),
            json!({
                "id": "1",
                "title": "Test Zaak",
                "status": "open"
            }),
        );
        issues.insert(
            "2".to_string(),
            json!({
                "id": "2",
                "title": "Andere Zaak",
                "status": "closed"
            }),
        );

        let demo_event = generate_demo_event(&issues);
        assert!(demo_event.is_some());

        let event = demo_event.unwrap();
        assert_eq!(event["specversion"], "1.0");
        assert!(event.get("id").is_some());
        assert!(event.get("source").is_some());
        assert!(event.get("type").is_some());
        assert!(event.get("time").is_some());

        // Should use the json.commit event type
        let event_type = event["type"].as_str().unwrap();
        assert_eq!(event_type, "json.commit");
    }

    #[test]
    fn test_generate_demo_event_empty_issues() {
        let empty_issues = HashMap::new();
        let demo_event = generate_demo_event(&empty_issues);
        assert!(demo_event.is_none());
    }
}

/// Generate a random demo CloudEvent that modifies an existing issue
pub fn generate_demo_event(existing_issues: &HashMap<String, Value>) -> Option<Value> {
    if existing_issues.is_empty() {
        return None;
    }

    let issue_ids: Vec<&String> = existing_issues.keys().collect();
    let random_issue_id = issue_ids[fastrand::usize(..issue_ids.len())];

    // Choose random operation type
    let operation_type = fastrand::usize(0..100);

    match operation_type {
        0..50 => Some(generate_random_comment_event(random_issue_id)),
        50..65 => Some(generate_random_task_event(random_issue_id)),
        65..80 => Some(generate_random_planning_event(random_issue_id)),
        80..90 => Some(generate_random_document_event(random_issue_id)),
        _ => Some(generate_random_patch_event(random_issue_id)),
    }
}

/// Helper function to create a base CloudEvent structure
fn create_base_cloud_event(source: &str, subject: Option<&str>, datacontenttype: &str) -> Value {
    let mut event = json!({
        "specversion": "1.0",
        "id": Uuid::now_v7().to_string(),
        "type": EVENT_TYPE_JSON_COMMIT,
        "source": source,
        "time": Utc::now().to_rfc3339(),
        "datacontenttype": datacontenttype,
        "dataschema": JSON_COMMIT_SCHEMA
    });

    if let Some(subj) = subject {
        event["subject"] = json!(subj);
    }

    event
}

/// Helper function to create JSONCommit data payload
fn create_commit_data(
    resource_id: &str,
    resource_data: Option<&Value>,
    patch_data: Option<&Value>,
    resource_schema: &str,
) -> Value {
    let mut data = json!({
        "schema": resource_schema,
        "resource_id": resource_id
    });

    if let Some(resource_data) = resource_data {
        data["resource_data"] = resource_data.clone();
    }

    if let Some(patch_data) = patch_data {
        data["patch"] = patch_data.clone();
    }

    data
}

/// Helper function to create a complete CloudEvent with JSONCommit
fn create_cloud_event(
    source: &str,
    subject: Option<&str>,
    data_content_type: &str,
    resource_id: &str,
    resource_data: Option<&Value>,
    patch_data: Option<&Value>,
    resource_schema: &str,
) -> Value {
    let mut event = create_base_cloud_event(source, subject, data_content_type);
    let data = create_commit_data(resource_id, resource_data, patch_data, resource_schema);
    event["data"] = data;
    event
}

fn generate_random_patch_event(issue_id: &str) -> Value {
    let patch_operations = [
        json!({"status": "open"}),
        json!({"status": "in_behandeling"}),
        json!({"status": "wachtend_op_informatie"}),
        json!({"status": "in_beoordeling"}),
        json!({"status": "gereed_voor_besluit"}),
        json!({"status": "afgesloten", "resolution": "toegekend"}),
        json!({"status": "afgesloten", "resolution": "afgewezen"}),
        json!({"status": "afgesloten", "resolution": "ingetrokken"}),
        json!({"assignee": "alice@gemeente.nl"}),
        json!({"assignee": "bob@gemeente.nl"}),
        json!({"assignee": "specialist@gemeente.nl"}),
        json!({"assignee": null}),
        json!({"priority": "hoog"}),
        json!({"priority": "normaal"}),
        json!({"priority": "laag"}),
        json!({"category": "omgevingsvergunning"}),
        json!({"category": "melding_openbare_ruimte"}),
        json!({"category": "bijstandsverzoek"}),
        json!({"category": "woo_verzoek"}),
        json!({"category": "parkeervergunning"}),
        json!({"due_date": "2024-03-15"}),
        json!({"due_date": "2024-04-01"}),
        json!({"tags": ["urgent", "spoed"]}),
        json!({"tags": ["complex"]}),
        json!({"tags": ["externe_partij"]}),
        json!({"department": "omgeving_en_vergunningen"}),
        json!({"department": "publiekszaken"}),
        json!({"department": "sociale_zaken"}),
        json!({"department": "juridische_zaken"}),
    ];

    let random_patch = &patch_operations[fastrand::usize(..patch_operations.len())];
    generate_patch_event_with_data(issue_id, random_patch)
}

fn generate_patch_event_with_data(issue_id: &str, patch_data: &Value) -> Value {
    create_cloud_event(
        DEFAULT_SOURCE,
        Some(issue_id),
        CONTENT_TYPE_JSON,
        issue_id,
        None,
        Some(patch_data),
        ISSUE_SCHEMA,
    )
}

fn generate_create_event_with_data(
    issue_id: &str,
    title: &str,
    description: &str,
    assignee: Option<&str>,
) -> Value {
    let mut issue_data = json!({
        "id": issue_id,
        "title": title,
        "description": description,
        "status": "open"
    });

    if let Some(assignee_email) = assignee {
        issue_data["assignee"] = json!(assignee_email);
        // Add assignee to involved list
        issue_data["involved"] = json!([assignee_email]);
    } else {
        issue_data["involved"] = json!([]);
    }

    create_cloud_event(
        DEFAULT_SOURCE,
        Some(issue_id),
        CONTENT_TYPE_JSON,
        issue_id,
        Some(&issue_data),
        None,
        ISSUE_SCHEMA,
    )
}

fn generate_delete_event_with_data(issue_id: &str, reason: &str) -> Value {
    // For deletion, we use the deleted field directly
    // When deleted is true, the entire resource is removed from the store
    let delete_data = json!({
        "schema": ISSUE_SCHEMA,
        "resource_id": issue_id,
        "actor": "system@gemeente.nl",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "deleted": true,
        "deletion_reason": reason
    });

    create_cloud_event(
        DEFAULT_SOURCE,
        Some(issue_id),
        CONTENT_TYPE_JSON,
        issue_id,
        Some(&delete_data),
        None,
        ISSUE_SCHEMA,
    )
}

fn generate_random_comment_event(issue_id: &str) -> Value {
    let official_comments = [
        "Status update: bezig met verwerking van deze aanvraag.",
        "Aanvullende informatie ontvangen van aanvrager.",
        "Doorverwezen naar de juiste afdeling voor behandeling.",
        "Locatie inspectie gepland voor volgende week.",
        "Advies gevraagd aan externe adviseur.",
        "Alle benodigde documenten zijn nu compleet.",
        "Zaak is in behandeling genomen door specialist.",
        "Eerste beoordeling van de aanvraag afgerond.",
        "Contact opgenomen met betrokken partijen.",
        "Verdere analyse van de situatie vereist.",
        "Planning gemaakt voor vervolgstappen.",
        "Overleg gepland met collega's over deze zaak.",
        "Termijn verlengd na overleg met aanvrager.",
        "Advies van juridische afdeling ingewonomen.",
        "Technische beoordeling uitgevoerd ter plaatse.",
        "Afspraak ingepland met aanvrager voor volgende week.",
        "Zaak doorgestuurd naar behandelend ambtenaar.",
        "Extra documentatie opgevraagd bij externe partij.",
        "Interne afstemming afgerond, kan door naar volgende fase.",
        "Wachten op goedkeuring van leidinggevende.",
        "Controle uitgevoerd, alles in orde bevonden.",
        "Vraag gesteld aan ICT-afdeling over technische aspecten.",
        "Besluitvorming uitgesteld tot na vakantieperiode.",
        "Prioriteit verhoogd vanwege urgentie van de aanvraag.",
        "Aanvraag gemarkeerd voor extra aandacht van senior medewerker.",
        "Update: wachten op reactie van externe instantie.",
        "Telefonisch contact gehad met aanvrager over voortgang.",
        "Zaak tijdelijk on-hold gezet vanwege onduidelijkheden.",
        "Herziening van de aanvraag na nieuwe informatie.",
        "Consultant ingeschakeld voor specialistisch advies.",
    ];

    let citizen_comments = [
        "Hallo waarom duurt dit zo lang? Het is al 3 weken geleden!",
        "Kan iemand mij uitleggen wat er gebeurt met mijn aanvraag?",
        "Ik heb nog steeds niks gehoord... Is er iemand die dit oppakt?",
        "Dit is belachelijk, waarom duurt alles bij de gemeente zo lang?",
        "Wanneer kan ik eindelijk een reactie verwachten?",
        "Mijn buurman had hetzelfde probleem en die kreeg binnen een week antwoord!",
        "Kunnen jullie niet wat sneller werken? Ik heb deadline!",
        "Is er überhaupt iemand die naar mijn zaak kijkt?",
        "Vriendelijk verzoek om even te laten weten wat de status is 🙏",
        "Help! Ik word gek van het wachten, wat gebeurt er nou?",
        "Ik begrijp er helemaal niks van... kan iemand uitleg geven?",
        "Dit is toch niet normaal? Zo lang wachten voor een simpele aanvraag?",
        "Volgens de website zou dit binnen 2 weken afgehandeld zijn...",
        "Ik ga een klacht indienen als dit niet snel opgelost wordt!",
        "Hoe moeilijk kan het zijn om even te reageren?",
        "Mijn geduld raakt op... wanneer krijg ik nou antwoord?",
        "Dit is de 5e keer dat ik contact opneem. HELP!",
        "Ik snap er niks van. Waarom is dit zo ingewikkeld?",
        "Kan er niet gewoon even iemand bellen om dit uit te leggen?",
        "Dit is echt frustrerend... ik wacht al maanden!",
    ];

    let official_actors = [
        "alice@gemeente.nl",
        "bob@gemeente.nl",
        "carol@gemeente.nl",
        "demo@gemeente.nl",
        "specialist@gemeente.nl",
    ];

    let citizen_actors = [
        "pietjansen@hotmail.com",
        "marieke.de.vries@gmail.com",
        "jan.klaassen@ziggo.nl",
        "a.peters@live.nl",
        "kees.van.dijk@kpn.nl",
        "susan.bakker@yahoo.com",
        "henk.groot@planet.nl",
        "annemarie@xs4all.nl",
    ];

    // 70% chance of citizen comment to make them more common
    let is_citizen_comment = fastrand::usize(0..100) < 70;

    let (comment_text, actor) = if is_citizen_comment {
        let comment = citizen_comments[fastrand::usize(..citizen_comments.len())];
        let actor = citizen_actors[fastrand::usize(..citizen_actors.len())];
        (comment, actor)
    } else {
        let comment = official_comments[fastrand::usize(..official_comments.len())];
        let actor = official_actors[fastrand::usize(..official_actors.len())];
        (comment, actor)
    };

    generate_comment_event_with_data(issue_id, comment_text, actor)
}

fn generate_comment_event_with_data(issue_id: &str, content: &str, actor: &str) -> Value {
    let comment_id = format!("comment-{}", Uuid::now_v7().simple());
    let comment_data = json!({
        "id": comment_id,
        "content": content,
        "author": actor,
        "parent_id": null,
        "mentions": []
    });

    let mut event = create_cloud_event(
        DEFAULT_SOURCE,
        Some(issue_id),
        CONTENT_TYPE_JSON,
        &comment_id,
        Some(&comment_data),
        None,
        COMMENT_SCHEMA,
    );

    // Add actor and timestamp to data level for timeline display
    if let Some(data) = event.get_mut("data") {
        data["actor"] = json!(actor);
        data["timestamp"] = json!(Utc::now().to_rfc3339());
    }

    event
}

/// Generate a random task timeline item for an issue
fn generate_random_task_event(issue_id: &str) -> Value {
    let tasks = [
        (
            "Documenten Controleren",
            "Controleer de ingediende documenten op volledigheid",
            "/review/documents",
        ),
        (
            "Afspraak Inplannen",
            "Plan een afspraak in met de aanvrager",
            "/schedule/appointment",
        ),
        (
            "Locatie Inspecteren",
            "Voer een inspectie ter plaatse uit",
            "/inspect/location",
        ),
        (
            "Aanvrager Bellen",
            "Bel de aanvrager voor aanvullende informatie",
            "/contact/applicant",
        ),
        (
            "Ontbrekende Documenten",
            "Upload ontbrekende documentatie naar het systeem",
            "/upload/documents",
        ),
        (
            "Juridische Controle",
            "Laat deze zaak controleren door de juridische afdeling",
            "/review/legal",
        ),
        (
            "Betaling Verwerken",
            "Verwerk de betaling voor leges",
            "/payment/process",
        ),
        (
            "Melding Versturen",
            "Stuur statusupdate naar aanvrager",
            "/send/notification",
        ),
        (
            "Eindcontrole aanvragen",
            "Voer eindcontrole uit voordat de zaak wordt afgerond",
            "/check/final",
        ),
    ];

    let actors = [
        "system@gemeente.nl",
        "workflow@gemeente.nl",
        "alice@gemeente.nl",
        "bob@gemeente.nl",
    ];

    let (cta, description, url) = tasks[fastrand::usize(..tasks.len())];
    let actor = actors[fastrand::usize(..actors.len())];

    generate_task_event_with_data(issue_id, cta, description, url, actor)
}

fn generate_task_event_with_data(
    issue_id: &str,
    cta: &str,
    description: &str,
    url: &str,
    actor: &str,
) -> Value {
    // Generate a deadline 1-5 days from now
    let days_ahead = fastrand::usize(1..=5);
    let deadline = (Utc::now() + Duration::days(days_ahead as i64))
        .format("%Y-%m-%d")
        .to_string();

    let task_id = format!("task-{}", Uuid::now_v7().simple());
    let task_data = json!({
        "id": task_id,
        "cta": cta,
        "description": description,
        "url": url,
        "completed": false,
        "deadline": deadline
    });

    let mut event = create_cloud_event(
        DEFAULT_SOURCE,
        Some(issue_id),
        CONTENT_TYPE_JSON,
        &task_id,
        Some(&task_data),
        None,
        TASK_SCHEMA,
    );

    // Add actor and timestamp to data level
    if let Some(data) = event.get_mut("data") {
        data["actor"] = json!(actor);
        data["timestamp"] = json!(Utc::now().to_rfc3339());
    }

    event
}

/// Generate a random planning event for an issue
fn generate_random_planning_event(issue_id: &str) -> Value {
    let plannings = [
        (
            "Vergunningsprocedure",
            "Proces voor het verkrijgen van de benodigde vergunningen",
            vec![
                ("Aanvraag indienen", "completed"),
                ("Behandeling door gemeente", "current"),
                ("Besluit gemeente", "planned"),
                ("Bezwaarperiode", "planned"),
                ("Vergunning geldig", "planned"),
            ],
        ),
        (
            "Verhuisprocedure",
            "Bij het verhuizen houden we ons aan de regels en richtlijnen van de gemeente.",
            vec![
                ("Doorgeven adreswijziging", "completed"),
                ("Update kadaster", "current"),
                ("Update gemeentedata", "planned"),
                ("Diensten wijzigen", "planned"),
                ("Informeren nieuwe bewoners", "planned"),
            ],
        ),
        (
            "Juridische procedure",
            "Stappen in de juridische behandeling",
            vec![
                ("Intake", "completed"),
                ("Onderzoek", "completed"),
                ("Advies opstellen", "current"),
                ("Besluitvorming", "planned"),
                ("Communicatie besluit", "planned"),
            ],
        ),
    ];

    let actors = [
        "specialist@gemeente.nl",
        "projectleider@gemeente.nl",
        "juridisch@gemeente.nl",
        "vergunningen@gemeente.nl",
    ];

    let (title, description, moments_data) = &plannings[fastrand::usize(..plannings.len())];
    let actor = actors[fastrand::usize(..actors.len())];

    generate_planning_event_with_data(issue_id, title, description, moments_data, actor)
}

fn generate_planning_event_with_data(
    issue_id: &str,
    title: &str,
    description: &str,
    moments_data: &Vec<(&str, &str)>,
    actor: &str,
) -> Value {
    // Generate moments with dates spread over the next few months
    let mut moments = Vec::new();
    let base_date = Utc::now();

    for (index, (moment_title, status)) in moments_data.iter().enumerate() {
        let days_offset = match status {
            &"completed" => -(fastrand::i64(5..=30)), // Past dates
            &"current" => fastrand::i64(-2..=2),      // Around now
            _ => fastrand::i64(7..=(30 + index as i64 * 14)), // Future dates
        };

        let moment_date = (base_date + Duration::days(days_offset))
            .format("%Y-%m-%d")
            .to_string();

        moments.push(json!({
            "id": format!("moment-{}", Uuid::now_v7().simple()),
            "date": moment_date,
            "title": moment_title,
            "status": status
        }));
    }

    let planning_id = format!("planning-{}", Uuid::now_v7().simple());
    let planning_data = json!({
        "id": planning_id,
        "title": title,
        "description": description,
        "moments": moments
    });

    let mut event = create_cloud_event(
        DEFAULT_SOURCE,
        Some(issue_id),
        CONTENT_TYPE_JSON,
        &planning_id,
        Some(&planning_data),
        None,
        PLANNING_SCHEMA,
    );

    // Add actor and timestamp to data level
    if let Some(data) = event.get_mut("data") {
        data["actor"] = json!(actor);
        data["timestamp"] = json!(Utc::now().to_rfc3339());
    }

    event
}

fn generate_random_document_event(issue_id: &str) -> Value {
    let documents = [
        ("Identiteitsbewijs_Kopie.pdf", 245632),
        ("Aanvraagformulier_Ingevuld.pdf", 512847),
        ("Bewijs_van_Adres.pdf", 189234),
        ("Ondertekende_Verklaring.pdf", 156789),
        ("Bijlage_Documenten.zip", 2847391),
        ("Paspoortfoto_Officieel.jpg", 89765),
        ("Uittreksel_BRP.pdf", 234567),
        ("Bewijs_Inkomen.pdf", 445823),
        ("Medische_Verklaring.pdf", 298374),
        ("Technische_Tekening.dwg", 1847563),
        ("Rapport_Onderzoek.docx", 756432),
        ("Overzicht_Kosten.xlsx", 98234),
    ];

    let actors = [
        "aanvrager@example.com",
        "alice@gemeente.nl",
        "bob@gemeente.nl",
        "specialist@gemeente.nl",
        "archief@gemeente.nl",
    ];

    let (title, size) = documents[fastrand::usize(..documents.len())];
    let actor = actors[fastrand::usize(..actors.len())];

    generate_document_event_with_data(issue_id, title, size, actor)
}

fn generate_document_event_with_data(issue_id: &str, title: &str, size: u64, actor: &str) -> Value {
    let document_id = format!("doc-{}", Uuid::now_v7().simple());
    let url = format!("https://example.com/documents/{}", document_id);
    let document_data = json!({
        "id": document_id,
        "title": title,
        "url": url,
        "size": size
    });

    let mut event = create_cloud_event(
        DEFAULT_SOURCE,
        Some(issue_id),
        CONTENT_TYPE_JSON,
        &document_id,
        Some(&document_data),
        None,
        DOCUMENT_SCHEMA,
    );

    // Add actor and timestamp to data level
    if let Some(data) = event.get_mut("data") {
        data["actor"] = json!(actor);
        data["timestamp"] = json!(Utc::now().to_rfc3339());
    }

    event
}

/// Convert a JSON CloudEvent to a CloudEvent struct
pub fn json_to_cloudevent(json_event: &Value) -> Option<CloudEvent> {
    Some(CloudEvent {
        specversion: json_event.get("specversion")?.as_str()?.to_string(),
        id: json_event.get("id")?.as_str()?.to_string(),
        source: json_event.get("source")?.as_str()?.to_string(),
        subject: json_event
            .get("subject")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string()),
        event_type: json_event.get("type")?.as_str()?.to_string(),
        time: json_event
            .get("time")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string()),
        datacontenttype: json_event
            .get("datacontenttype")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string()),
        dataschema: json_event
            .get("dataschema")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string()),
        dataref: json_event
            .get("dataref")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string()),
        sequence: json_event
            .get("sequence")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string()),
        sequencetype: json_event
            .get("sequencetype")
            .and_then(|t| t.as_str())
            .map(|s| s.to_string()),
        data: json_event.get("data").cloned(),
    })
}
