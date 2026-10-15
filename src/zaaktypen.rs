use crate::schemas::{CloudEvent, JSONCommit};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct ZaakTypeDefinition {
    pub id: String,
    pub version: String,
    pub title: String,
    pub statuses: Vec<ZaakStatus>,
    #[serde(default)]
    pub task_templates: Vec<TaskTemplate>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZaakStatus {
    pub id: String,
    pub title: String,
    pub sequence: u32,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaskTemplate {
    pub id: String,
    pub title: String,
    pub question: String,
    pub status: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct DerivedZaakEvent {
    pub event: CloudEvent,
    pub resource_type: &'static str,
    pub resource_id: String,
    pub resource: Value,
}

pub fn load_definition(
    zaaktype_ref: &str,
) -> Result<ZaakTypeDefinition, Box<dyn std::error::Error + Send + Sync>> {
    let (id, version) = parse_zaaktype_ref(zaaktype_ref);
    let path = definition_path(&id, &version)?;
    let text = std::fs::read_to_string(&path)?;
    let definition = serde_json::from_str(&text)?;
    Ok(definition)
}

pub fn derive_events_for_issue(
    issue_id: &str,
    issue: &Value,
    actor: &str,
) -> Result<Vec<DerivedZaakEvent>, Box<dyn std::error::Error + Send + Sync>> {
    let Some(zaaktype_ref) = issue.get("zaaktype").and_then(Value::as_str) else {
        return Ok(Vec::new());
    };

    let definition = load_definition(zaaktype_ref)?;
    if definition.statuses.is_empty() {
        return Ok(Vec::new());
    }

    let current_status_id = issue
        .get("current_status_id")
        .and_then(Value::as_str)
        .filter(|status_id| definition.statuses.iter().any(|status| status.id == *status_id))
        .unwrap_or(&definition.statuses[0].id)
        .to_string();

    let mut derived = Vec::new();

    if issue.get("current_status_id").and_then(Value::as_str) != Some(current_status_id.as_str()) {
        let resource = json!({ "current_status_id": current_status_id });
        derived.push(json_commit_event(
            issue_id,
            issue_id,
            "Issue",
            actor,
            resource.clone(),
            true,
            "zaaktype-engine-status-init",
            "Issue",
        ));
    }

    let planning_id = format!("{}::zaaktype-planning", issue_id);
    let planning = planning_for_definition(&definition, &current_status_id);
    derived.push(json_commit_event(
        issue_id,
        &planning_id,
        "Planning",
        actor,
        planning.clone(),
        false,
        "zaaktype-engine-planning",
        "Planning",
    ));

    for template in definition
        .task_templates
        .iter()
        .filter(|template| template.status == current_status_id)
    {
        let task_id = format!(
            "{}::zaaktype-task::{}::{}",
            issue_id, current_status_id, template.id
        );
        let task = json!({
            "cta": template.title,
            "description": task_description(template),
            "url": format!("/zaak/{}", issue_id),
            "completed": false,
            "zaaktype": format!("{}@{}", definition.id, definition.version),
            "zaaktype_status_id": current_status_id,
            "zaaktype_task_template_id": template.id,
            "required": template.required
        });
        derived.push(json_commit_event(
            issue_id,
            &task_id,
            "Task",
            actor,
            task,
            false,
            "zaaktype-engine-task",
            "Task",
        ));
    }

    Ok(derived)
}

fn json_commit_event(
    subject: &str,
    resource_id: &str,
    schema_name: &str,
    actor: &str,
    resource: Value,
    is_patch: bool,
    source: &str,
    resource_type: &'static str,
) -> DerivedZaakEvent {
    let event_id = uuid::Uuid::now_v7().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let data = if is_patch {
        serde_json::to_value(JSONCommit {
            schema: format!("https://zaakchat.nl/schemas/{}", schema_name),
            resource_id: resource_id.to_string(),
            actor: actor.to_string(),
            timestamp: Some(now.clone()),
            resource_data: None,
            patch: Some(resource.clone()),
            deleted: None,
        })
        .expect("JSONCommit should serialize")
    } else {
        serde_json::to_value(JSONCommit {
            schema: format!("https://zaakchat.nl/schemas/{}", schema_name),
            resource_id: resource_id.to_string(),
            actor: actor.to_string(),
            timestamp: Some(now.clone()),
            resource_data: Some(resource.clone()),
            patch: None,
            deleted: None,
        })
        .expect("JSONCommit should serialize")
    };

    DerivedZaakEvent {
        event: CloudEvent {
            specversion: "1.0".to_string(),
            id: event_id,
            source: source.to_string(),
            subject: subject.to_string(),
            event_type: "json.commit".to_string(),
            time: Some(now),
            datacontenttype: Some("application/json".to_string()),
            dataschema: Some("https://zaakchat.nl/schemas/JSONCommit".to_string()),
            dataref: None,
            sequence: None,
            sequencetype: None,
            data: Some(data),
        },
        resource_type,
        resource_id: resource_id.to_string(),
        resource,
    }
}

fn planning_for_definition(definition: &ZaakTypeDefinition, current_status_id: &str) -> Value {
    let current_sequence = definition
        .statuses
        .iter()
        .find(|status| status.id == current_status_id)
        .map(|status| status.sequence)
        .unwrap_or(1);

    let moments = definition
        .statuses
        .iter()
        .map(|status| {
            let planning_status = if status.sequence < current_sequence {
                "completed"
            } else if status.sequence == current_sequence {
                "current"
            } else {
                "planned"
            };
            json!({
                "id": status.id,
                "title": status.title,
                "status": planning_status
            })
        })
        .collect::<Vec<_>>();

    json!({
        "title": definition.title,
        "description": format!("Proces op basis van zaaktype {}@{}", definition.id, definition.version),
        "moments": moments,
        "zaaktype": format!("{}@{}", definition.id, definition.version)
    })
}

fn task_description(template: &TaskTemplate) -> String {
    if template.description.is_empty() {
        template.question.clone()
    } else {
        format!("{}\n\n{}", template.question, template.description)
    }
}

fn parse_zaaktype_ref(reference: &str) -> (String, String) {
    match reference.rsplit_once('@') {
        Some((id, version)) => (id.to_string(), version.to_string()),
        None => (reference.to_string(), "1.9".to_string()),
    }
}

fn definition_path(
    id: &str,
    version: &str,
) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let file_name = format!("{}-{}.json", id.replace('/', "-"), version);
    let candidates = [
        Path::new("zaaktypen").join("generated").join(&file_name),
        Path::new("/app").join("zaaktypen").join("generated").join(&file_name),
    ];

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(format!("zaaktype definition not found for {}@{}", id, version).into())
}
