//! HTTP handlers for /events, /resources, and /query endpoints

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
    response::{IntoResponse, Response},
    Json,
};
use futures_util::stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::schemas::{CloudEvent, JSONCommit};
use crate::storage::{SearchResult, Storage};
use crate::types::PushSubscription;

/// Shared application state with storage (handlers view)
///
/// This AppState includes a reference to the search index manager so search
/// operations are handled by the dedicated search subsystem (`src/search.rs`).
/// It also contains the push subscription store so push-related handlers
/// (subscribe/unsubscribe) can access and mutate subscriptions when needed.
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<Storage>,
    /// Search index manager (separate module handling Tantivy)
    pub search: Arc<crate::search::SearchIndex>,
    pub tx: tokio::sync::broadcast::Sender<CloudEvent>,
    /// Push notification subscriptions (shared across handlers)
    pub push_subscriptions: Arc<tokio::sync::RwLock<Vec<PushSubscription>>>,
}

/// Convenience constructor for handlers to create an AppState when needed.
impl AppState {
    pub fn new(
        storage: Arc<Storage>,
        search: Arc<crate::search::SearchIndex>,
        tx: tokio::sync::broadcast::Sender<CloudEvent>,
    ) -> Self {
        Self {
            storage,
            search,
            tx,
            push_subscriptions: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }
}

/// Response for resource retrieval
#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceResponse {
    pub id: String,
    pub resource_type: String,
    pub data: Value,
}

/// Query parameters for listing resources
#[derive(Debug, Deserialize)]
pub struct ListParams {
    #[serde(default = "default_offset")]
    pub offset: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_offset() -> usize {
    0
}

fn default_limit() -> usize {
    10000
}

/// Query parameters for search
#[derive(Debug, Deserialize)]
pub struct QueryParams {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

/// Query parameters for listing events (used for JSON listing or snapshot pagination)
/// Query parameters for listing events (used for JSON listing or snapshot pagination)
#[derive(Debug, Deserialize)]
pub struct EventsListParams {
    #[serde(default = "default_offset")]
    pub offset: usize,
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Optional topic filter (matches subject or event type)
    #[serde(default)]
    pub topic: Option<String>,
    /// Optional format hint (e.g. "json"). When "json" is set, the handler will return JSON rather than SSE.
    #[serde(default)]
    pub format: Option<String>,
    /// Optional sequence key to fetch events after (zero-padded sequence string).
    /// Example: "00000000000000000042"
    #[serde(default)]
    pub after_seq: Option<String>,
}

/// GET /events - Returns an SSE stream by default. If the query `?format=json` is present,
/// the handler will return a JSON list instead (keeps frontend compatibility: SSE is default).
pub async fn get_or_stream_events(
    State(state): State<AppState>,
    _headers: HeaderMap,
    Query(params): Query<EventsListParams>,
) -> Result<Response, StatusCode> {
    // Only return JSON when explicitly requested via query param `?format=json`.
    let want_json = params
        .format
        .as_deref()
        .map(|s| s.eq_ignore_ascii_case("json"))
        .unwrap_or(false);

    if want_json {
        // Return JSON listing (paginated + optional topic filter)
        let events = state
            .storage
            .list_events_after(params.after_seq.clone(), params.limit)
            .await
            .map_err(|e| {
                eprintln!("Failed to list events: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Filter events by topic if provided
        let filtered: Vec<CloudEvent> = if let Some(topic) = params.topic.as_deref() {
            events
                .into_iter()
                .filter(|e| {
                    e.subject
                        .as_deref()
                        .map(|s| s.contains(topic))
                        .unwrap_or(false)
                        || e.event_type.contains(topic)
                })
                .collect()
        } else {
            events
        };

        // Keep the order as provided by storage (earliest-first). No reversal applied here.

        return Ok(Json(filtered).into_response());
    }

    // Default: return SSE stream (snapshot followed by deltas)
    let rx = state.tx.subscribe();

    // Use storage to build a snapshot (paginated)
    let snapshot_events = state
        .storage
        .list_events_after(params.after_seq.clone(), params.limit)
        .await
        .map_err(|e| {
            eprintln!("Failed to build snapshot events: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Keep snapshot order as provided by storage (earliest-first). No reversal applied here.

    let snapshot = serde_json::to_string(&snapshot_events).unwrap_or_else(|_| "[]".to_string());

    let stream = stream::once(async move {
        Ok::<Event, Infallible>(Event::default().event("snapshot").data(snapshot))
    })
    .chain(
        BroadcastStream::new(rx)
            .map(|msg| {
                let delta = msg.unwrap();
                let json = serde_json::to_string(&delta).unwrap_or_else(|_| "{}".to_string());
                Event::default().event("delta").data(json)
            })
            .map(Ok),
    );

    let sse = Sse::new(stream).keep_alive(KeepAlive::default());
    Ok(sse.into_response())
}

/// Response for query endpoint
#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub count: usize,
}

/// Error response type
#[derive(Debug, Serialize)]
#[allow(dead_code)]
pub struct ErrorResponse {
    pub error: String,
}

/// POST /events - Handle incoming CloudEvents (Command + Sync)
/// This is where resources are created, updated, and deleted
pub async fn handle_event(
    State(state): State<AppState>,
    Json(mut event): Json<CloudEvent>,
) -> Result<Response, StatusCode> {
    // Store the event and get the assigned server sequence key
    let seq_key = state.storage.store_event(&event).await.map_err(|e| {
        eprintln!("Failed to store event: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Attach the assigned sequence to the CloudEvent so clients can use it for ordering/pagination
    event.sequence = Some(seq_key.clone());

    // Schedule background indexing of the event (search subsystem).
    // Serialize once and pass the payload string to avoid cloning the entire CloudEvent.
    {
        let search = state.search.clone();
        // Serialize CloudEvent once (no snippet content to avoid extra allocations)
        let payload = serde_json::to_string(&event).unwrap_or_default();
        let id = event.id.clone();
        let doc_type = event.event_type.clone();
        // Do not parse timestamp here; pass None to the search indexer (it can set now)
        tokio::spawn(async move {
            if let Err(e) = search
                .add_event_payload(&id, &doc_type, "", &payload, None)
                .await
            {
                eprintln!(
                    "[handlers][bg] failed adding event payload to search index id={} err={}",
                    id, e
                );
            } else {
                println!(
                    "[handlers][bg] scheduled event payload added to search index id={}",
                    id
                );
            }
        });
    }

    // Process the event to update resources
    if let Err(e) = process_event(&state, &event).await {
        eprintln!("Failed to process event: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Broadcast the event (with attached sequence) to SSE subscribers
    let _ = state.tx.send(event.clone());

    Ok((StatusCode::ACCEPTED, Json(event)).into_response())
}

/// Process an event and update resources accordingly
pub async fn process_event(
    state: &AppState,
    event: &CloudEvent,
) -> Result<(), Box<dyn std::error::Error>> {
    // Extract data from the event
    let data = match &event.data {
        Some(d) => d,
        None => return Ok(()), // No data to process
    };

    // Check if this is a JSONCommit event (accept both legacy and NL-VNG names)
    if event.event_type == "nl.vng.zaken.json-commit.v1" || event.event_type == "json.commit" {
        // Log the incoming event and data shape for diagnostics
        println!(
            "[handlers] processing json-commit event id={} subject={:?} source={}",
            event.id, event.subject, event.source
        );

        let commit: JSONCommit = serde_json::from_value(data.clone())?;

        // Log commit details that matter for resource creation
        println!(
            "[handlers] commit parsed: resource_id={} schema={} resource_data_present={} patch_present={}",
            commit.resource_id,
            commit.schema,
            commit.resource_data.is_some(),
            commit.patch.is_some()
        );

        // Handle deletion
        if commit.deleted.unwrap_or(false) {
            println!("[handlers] deleting resource id={}", commit.resource_id);
            state.storage.delete_resource(&commit.resource_id).await?;
            return Ok(());
        }

        // Determine resource type more robustly:
        // 1. Prefer schema hint (if it contains known type names)
        // 2. Fallback to subject hint
        // 3. Inspect resource_data keys (title/content/cta/moments/url)
        // 4. Otherwise "unknown"
        let mut resource_type = extract_resource_type_from_schema(&commit.schema).to_string();

        if resource_type == "unknown" {
            if let Some(subject) = &event.subject {
                let subj_type = extract_resource_type_from_subject(subject);
                if subj_type != "unknown" {
                    resource_type = subj_type.to_string();
                }
            }
        }

        if resource_type == "unknown" {
            if let Some(resource_data) = &commit.resource_data {
                if resource_data.is_object() {
                    let obj = resource_data.as_object().unwrap();
                    if obj.contains_key("title") {
                        resource_type = "issue".to_string();
                    } else if obj.contains_key("content") {
                        resource_type = "comment".to_string();
                    } else if obj.contains_key("cta") {
                        resource_type = "task".to_string();
                    } else if obj.contains_key("moments") {
                        resource_type = "planning".to_string();
                    } else if obj.get("url").is_some() || obj.get("size").is_some() {
                        resource_type = "document".to_string();
                    }
                }
            }
        }

        // Final diagnostics before storing
        println!(
            "[handlers] storing resource id={} determined_type={} resource_data_present={}",
            commit.resource_id,
            resource_type,
            commit.resource_data.is_some()
        );

        // Get existing resource if it exists
        let existing_resource = state.storage.get_resource(&commit.resource_id).await?;

        // Apply changes (merge patch or replace with resource_data)
        let new_resource = if let Some(mut existing) = existing_resource {
            // Apply patch if provided
            if let Some(patch) = &commit.patch {
                println!(
                    "[handlers] applying patch to existing resource id={}",
                    commit.resource_id
                );
                apply_json_merge_patch(&mut existing, patch);
            }
            // Override with full resource_data if provided
            if let Some(resource_data) = &commit.resource_data {
                println!(
                    "[handlers] replacing existing resource id={} with provided resource_data",
                    commit.resource_id
                );
                existing = resource_data.clone();
            }
            existing
        } else {
            // New resource - use resource_data if available, else empty object
            commit
                .resource_data
                .clone()
                .unwrap_or_else(|| serde_json::json!({}))
        };

        // Store the updated resource
        if let Err(e) = state
            .storage
            .store_resource(&commit.resource_id, &resource_type, &new_resource)
            .await
        {
            eprintln!(
                "[handlers] failed to store resource id={} type={} error={}",
                commit.resource_id, resource_type, e
            );
            return Err(e);
        } else {
            println!(
                "[handlers] successfully stored resource id={} type={}",
                commit.resource_id, resource_type
            );

            // Schedule background indexing of the resource via the search subsystem.
            // Serialize the resource once and pass the payload string to avoid heavy cloning.
            let resource_id = commit.resource_id.clone();
            let resource_type_clone = resource_type.clone();
            let data_clone = new_resource.clone();
            let search = state.search.clone();

            // Use commit.timestamp if available (try to parse), otherwise pass None.
            let timestamp_opt = commit
                .timestamp
                .as_ref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc));

            // Serialize payload once (avoid creating an additional textual snippet)
            let payload = serde_json::to_string(&data_clone).unwrap_or_default();

            tokio::spawn(async move {
                if let Err(err) = search
                    .add_resource_payload(
                        &resource_id,
                        &resource_type_clone,
                        "",
                        &payload,
                        timestamp_opt,
                    )
                    .await
                {
                    eprintln!(
                        "[handlers][bg] failed adding resource payload to search index id={} err={}",
                        resource_id, err
                    );
                } else {
                    println!(
                        "[handlers][bg] scheduled resource payload added to search index id={}",
                        resource_id
                    );
                }
            });
        }
    } else {
        // For other event types, you might want to handle them differently
        // For now, we'll just store them as-is if a subject exists
        if let Some(subject) = &event.subject {
            let resource_type = extract_resource_type_from_subject(subject);
            println!(
                "[handlers] non-json-commit event: storing event.id={} as resource type={}",
                event.id, resource_type
            );
            // store resource
            state
                .storage
                .store_resource(&event.id, resource_type, data)
                .await?;

            // schedule resource indexing via search subsystem (serialize once)
            let id_clone = event.id.clone();
            let rt_clone = resource_type.to_string();
            let data_clone = data.clone();
            let payload = serde_json::to_string(&data_clone).unwrap_or_default();
            let search = state.search.clone();
            tokio::spawn(async move {
                if let Err(err) = search
                    .add_resource_payload(&id_clone, &rt_clone, "", &payload, None)
                    .await
                {
                    eprintln!(
                        "[handlers][bg] failed adding non-json-commit resource payload id={} err={}",
                        id_clone, err
                    );
                } else {
                    println!(
                        "[handlers][bg] scheduled non-json-commit resource payload added to search index id={}",
                        id_clone
                    );
                }
            });
        } else {
            println!(
                "[handlers] non-json-commit event without subject: event.id={}",
                event.id
            );
        }
    }

    Ok(())
}

/// Extract resource type from schema URL
fn extract_resource_type_from_schema(schema: &str) -> &str {
    if schema.contains("Issue") {
        "issue"
    } else if schema.contains("Comment") {
        "comment"
    } else if schema.contains("Task") {
        "task"
    } else if schema.contains("Planning") {
        "planning"
    } else if schema.contains("Document") {
        "document"
    } else {
        "unknown"
    }
}

/// Extract resource type from subject
fn extract_resource_type_from_subject(subject: &str) -> &str {
    if subject.contains("issue") {
        "issue"
    } else if subject.contains("comment") {
        "comment"
    } else if subject.contains("task") {
        "task"
    } else if subject.contains("planning") {
        "planning"
    } else if subject.contains("document") {
        "document"
    } else {
        "unknown"
    }
}

/// Apply JSON Merge Patch (RFC 7396)
fn apply_json_merge_patch(target: &mut Value, patch: &Value) {
    if !patch.is_object() {
        *target = patch.clone();
        return;
    }

    if !target.is_object() {
        *target = serde_json::json!({});
    }

    let target_obj = target.as_object_mut().unwrap();
    let patch_obj = patch.as_object().unwrap();

    for (key, value) in patch_obj {
        if value.is_null() {
            target_obj.remove(key);
        } else if value.is_object() && target_obj.contains_key(key) {
            let mut target_value = target_obj.get(key).unwrap().clone();
            apply_json_merge_patch(&mut target_value, value);
            target_obj.insert(key.clone(), target_value);
        } else {
            target_obj.insert(key.clone(), value.clone());
        }
    }
}

/// GET /resources - List all resources (paginated)
pub async fn list_resources(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<ResourceResponse>>, StatusCode> {
    let resources = state
        .storage
        .list_resources(params.offset, params.limit)
        .await
        .map_err(|e| {
            eprintln!("Failed to list resources: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response: Vec<ResourceResponse> = resources
        .into_iter()
        .map(|(id, data)| {
            // Try to determine resource type from the data
            let resource_type = if let Some(_title) = data.get("title") {
                // Likely an issue
                "issue".to_string()
            } else if let Some(_content) = data.get("content") {
                "comment".to_string()
            } else if let Some(_cta) = data.get("cta") {
                "task".to_string()
            } else if let Some(_moments) = data.get("moments") {
                "planning".to_string()
            } else {
                "unknown".to_string()
            };

            ResourceResponse {
                id,
                resource_type,
                data,
            }
        })
        .collect();

    Ok(Json(response))
}

/// GET /resources/:id - Get a specific resource
pub async fn get_resource(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let resource = state.storage.get_resource(&id).await.map_err(|e| {
        eprintln!("Failed to get resource: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match resource {
        Some(data) => Ok(Json(data)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// DELETE /resources/:id - Delete a specific resource
pub async fn delete_resource(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    state.storage.delete_resource(&id).await.map_err(|e| {
        eprintln!("Failed to delete resource: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /query - Search resources using full-text search
/// Returns structured search results produced by the storage layer.
pub async fn query_resources(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SearchResult>>, StatusCode> {
    let results = state
        .search
        .search(&*state.storage, &params.q, params.limit)
        .await
        .map_err(|e| {
            eprintln!("Failed to search: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Return the structured results directly (each result may contain an `event` and/or `resource`).
    Ok(Json(results))
}

/// GET /debug/db - Return counts and sample ids of events and resources for diagnostics.
/// Use this to verify what is persisted on disk.
pub async fn debug_db(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Gather a reasonably sized sample (limit to avoid heavy work)
    let sample_limit = 50usize;

    // Events
    let events = state
        .storage
        .list_events(0, sample_limit)
        .await
        .map_err(|e| {
            eprintln!("Failed to list events for debug: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Resources
    let resources = state
        .storage
        .list_resources(0, sample_limit)
        .await
        .map_err(|e| {
            eprintln!("Failed to list resources for debug: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Build summaries
    let event_count = events.len();
    let resource_count = resources.len();
    let event_ids: Vec<String> = events.into_iter().map(|e| e.id).collect();
    let resource_ids: Vec<String> = resources.into_iter().map(|(id, _)| id).collect();

    let resp = serde_json::json!({
        "event_count": event_count,
        "resource_count": resource_count,
        "event_ids": event_ids,
        "resource_ids": resource_ids,
    });

    Ok(Json(resp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_json_merge_patch() {
        let mut target = serde_json::json!({
            "title": "Old Title",
            "status": "open",
            "nested": {
                "a": 1,
                "b": 2
            }
        });

        let patch = serde_json::json!({
            "title": "New Title",
            "status": null,
            "nested": {
                "b": 3,
                "c": 4
            }
        });

        apply_json_merge_patch(&mut target, &patch);

        assert_eq!(target["title"], "New Title");
        assert!(!target.as_object().unwrap().contains_key("status"));
        assert_eq!(target["nested"]["a"], 1);
        assert_eq!(target["nested"]["b"], 3);
        assert_eq!(target["nested"]["c"], 4);
    }

    #[test]
    fn test_extract_resource_type_from_schema() {
        assert_eq!(
            extract_resource_type_from_schema("http://example.com/Issue"),
            "issue"
        );
        assert_eq!(
            extract_resource_type_from_schema("http://example.com/Comment"),
            "comment"
        );
        assert_eq!(
            extract_resource_type_from_schema("http://example.com/Task"),
            "task"
        );
    }

    #[test]
    fn test_extract_resource_type_from_subject() {
        assert_eq!(extract_resource_type_from_subject("issue/123"), "issue");
        assert_eq!(extract_resource_type_from_subject("comment/456"), "comment");
        assert_eq!(extract_resource_type_from_subject("unknown/789"), "unknown");
    }
}
