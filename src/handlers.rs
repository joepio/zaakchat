//! HTTP handlers for /events, /resources, and /query endpoints

use crate::email::EmailService;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
    response::{IntoResponse, Response},
    Json,
};
use dashmap::DashMap;
use futures_util::stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::convert::Infallible;
use std::sync::Arc;
use std::time::{Duration, Instant};
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
    /// Email service for sending magic links and notifications
    pub email_service: Arc<EmailService>,
    /// Track active users for smart notification suppression
    pub active_users: Arc<DashMap<String, Instant>>,
}

/// Convenience constructor for handlers to create an AppState when needed.
impl AppState {
    pub fn new(
        storage: Arc<Storage>,
        search: Arc<crate::search::SearchIndex>,
        tx: tokio::sync::broadcast::Sender<CloudEvent>,
        email_service: Arc<EmailService>,
    ) -> Self {
        Self {
            storage,
            search,
            tx,
            push_subscriptions: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            email_service,
            active_users: Arc::new(DashMap::new()),
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
    /// Optional user identifier to scope the search (e.g. "alice@gemeente.nl")
    pub user: Option<String>,
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
    /// Optional JWT token for authentication (passed in query for SSE)
    #[serde(default)]
    pub token: Option<String>,
}

/// Helper to get all topics (issue IDs) a user has access to using Tantivy search.
/// This is much faster than checking access for each event individually (O(1) vs O(n) queries).
async fn get_authorized_topics(
    state: &AppState,
    user_id: &str,
) -> Result<std::collections::HashSet<String>, StatusCode> {
    // Query Tantivy for all issues where the user is involved
    // Tantivy supports nested JSON field queries: json_payload.involved:username
    // Note: We search for the username part only (before @) because Tantivy's tokenizer
    // splits on @ and other special characters
    let username = user_id.split('@').next().unwrap_or(user_id);
    let query = format!("json_payload.involved:{}", username);

    eprintln!(
        "[auth] Searching for authorized topics with query: {}",
        query
    );

    // Search with a high limit (we want all issues the user has access to)
    let results = state
        .search
        .search(&state.storage, &query, 10000)
        .await
        .map_err(|e| {
            eprintln!("[auth] Tantivy search failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    eprintln!("[auth] Tantivy returned {} results", results.len());

    // Extract resource IDs for issues only (filter by type)
    // SearchResult.id contains the resource ID for resource matches
    let mut topic_set = std::collections::HashSet::new();
    for result in results {
        // Check if this is an issue by looking at the resource data
        if let Some(resource) = &result.resource {
            if let Some(involved) = resource.get("involved").and_then(|v| v.as_array()) {
                // This is an issue (has involved array)
                // Check if user is actually in the involved list
                if involved.iter().any(|v| v.as_str() == Some(user_id)) {
                    topic_set.insert(result.id.clone());
                    eprintln!("[auth] Added authorized topic: {}", result.id);
                }
            }
        }
    }

    eprintln!("[auth] Total authorized topics: {}", topic_set.len());
    Ok(topic_set)
}

/// Helper to check if a user has access to a resource (and thus its events)
async fn check_access(storage: &Storage, user_id: &str, resource_id: &str) -> bool {
    // 1. Try to fetch the resource
    let resource = match storage.get_resource(resource_id).await {
        Ok(Some(r)) => r,
        _ => return false, // Resource not found or error -> deny
    };

    // 2. Determine type and check access
    // We can try to guess type from fields if not explicitly known, but storage stores it.
    // However, get_resource returns just the JSON Value.
    // We can inspect the value.

    // Check if it's an Issue
    if let Some(involved) = resource.get("involved").and_then(|v| v.as_array()) {
        // It's likely an Issue (or something with involved list)
        for person in involved {
            if person.as_str() == Some(user_id) {
                return true;
            }
        }
        eprintln!(
            "[auth] Access denied for user {} to resource {}. Involved: {:?}",
            user_id, resource_id, involved
        );
        return false;
    }

    // Check if it's a Comment (has quote_comment)
    if let Some(quote_id) = resource.get("quote_comment").and_then(|v| v.as_str()) {
        // Recursively check access on the quoted comment associated resource
        return Box::pin(check_access(storage, user_id, quote_id)).await;
    }

    // For other types (Task, Planning, Document), we need to know their parent.
    // If they don't have a parent link in the JSON, we can't authorize them based on Issue.
    // Current schema for Task/Planning/Document doesn't show a parent_id.
    // If they are standalone, we might default to deny or allow.
    // Given the strict requirement "only shows events where the topic is from an authenticated issue",
    // we should probably deny if we can't link it to an issue.
    // However, for the demo, maybe we assume they are open if not linked?
    // Or maybe we just return false to be safe.
    false
}

/// GET /events - Returns an SSE stream by default. If the query `?format=json` is present,
/// the handler will return a JSON list instead (keeps frontend compatibility: SSE is default).
pub async fn get_or_stream_events(
    State(state): State<AppState>,
    _headers: HeaderMap,
    Query(params): Query<EventsListParams>,
) -> Result<Response, StatusCode> {
    // 1. Authenticate
    let token = params.token.as_deref().ok_or(StatusCode::UNAUTHORIZED)?;
    let claims = crate::auth::verify_jwt(token).map_err(|_| StatusCode::UNAUTHORIZED)?;
    let user_id = claims.sub;

    // Update active status
    state.active_users.insert(user_id.clone(), Instant::now());

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

        // Filter events by topic AND authorization
        let mut filtered = Vec::new();
        for event in events {
            // Topic filter
            if let Some(topic) = params.topic.as_deref() {
                let matches = event.subject.contains(topic) || event.event_type.contains(topic);
                if !matches {
                    continue;
                }
            }

            // Authorization filter
            // Use subject as resource_id if available
            if check_access(&state.storage, &user_id, &event.subject).await {
                filtered.push(event);
            }
        }

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

    // OPTIMIZATION: Get all authorized topics at once using Tantivy (O(1) query)
    // instead of checking each event individually (O(n) queries)
    let authorized_topics = get_authorized_topics(&state, &user_id).await.map_err(|e| {
        eprintln!("Failed to get authorized topics: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Filter snapshot events using in-memory HashSet lookup (very fast!)
    let authorized_snapshot: Vec<_> = snapshot_events
        .into_iter()
        .filter(|event| {
            authorized_topics.contains(&event.subject) || event.event_type == "system.reset"
        })
        .collect();

    let snapshot = serde_json::to_string(&authorized_snapshot).unwrap_or_else(|_| "[]".to_string());

    let stream = stream::once(async move {
        Ok::<Event, Infallible>(Event::default().event("snapshot").data(snapshot))
    })
    .chain(
        BroadcastStream::new(rx)
            .then(move |msg| {
                let state_clone = state.clone();
                let user_id_clone = user_id.clone();
                let authorized_topics = authorized_topics.clone();
                async move {
                    // Update active status on every event check (keep-alive ish)
                    state_clone
                        .active_users
                        .insert(user_id_clone.clone(), Instant::now());

                    match msg {
                        Ok(event) => {
                            // Check authorization
                            // Optimization: use the static set first
                            if authorized_topics.contains(&event.subject)
                                || event.event_type == "system.reset"
                            {
                                return Some(event);
                            }

                            // Dynamic check for new issues or updated access
                            if check_access(&state_clone.storage, &user_id_clone, &event.subject)
                                .await
                            {
                                // Note: We can't easily update authorized_topics here as it's a cloned HashSet
                                // in a stream. But check_access is fast enough for the delta stream.
                                return Some(event);
                            }
                            None
                        }
                        Err(_) => None,
                    }
                }
            })
            .filter_map(|opt| opt)
            .map(|event| {
                let json = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
                Ok(Event::default().event("delta").data(json))
            }),
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
    // Index the event synchronously
    // Schedule background indexing of the event (search subsystem).
    // Serialize once and pass the payload string to avoid cloning the entire CloudEvent.
    // Index the event synchronously
    {
        let search = state.search.clone();
        // Serialize CloudEvent once (no snippet content to avoid extra allocations)
        let payload = serde_json::to_string(&event).unwrap_or_default();
        let id = event.id.clone();

        // Architecture Decision: All CloudEvents are indexed with doc_type="Event".
        // This allows searching the audit history via is:Event.
        // Specific event types (e.g. json.commit) are properties of the event payload.
        let doc_type = "Event".to_string();

        // Do not parse timestamp here; pass None to the search indexer (it can set now)

        if let Err(e) = search
            .add_event_payload(&id, &doc_type, "", &payload, None)
            .await
        {
            eprintln!(
                "[handlers] failed adding event payload to search index id={} err={}",
                id, e
            );
        }
    }

    // Process the event to update resources
    if let Err(e) = process_event(&state, &event).await {
        eprintln!("Failed to process event: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Force a commit to ensure the event is searchable immediately
    // This is critical for the "create then view" flow where the user expects
    // the new item to be available in the snapshot immediately.
    if let Err(e) = state.search.commit().await {
        eprintln!("[handlers] failed to commit search index: {}", e);
    }

    // Broadcast the event (with attached sequence) to SSE subscribers
    let _ = state.tx.send(event.clone());

    Ok((StatusCode::ACCEPTED, Json(event)).into_response())
}

/// Helper to send notifications for new comments/issues
async fn send_notifications_for_event(
    state: &AppState,
    event: &CloudEvent,
    resource: &Value,
    old_resource: Option<&Value>,
) {
    // 1. Determine if this is a notify-able event (new comment or issue)
    let resource_id = match resource.get("id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return,
    };

    // Determine type
    let is_comment = resource.get("content").is_some();
    let is_issue = resource.get("title").is_some() && resource.get("involved").is_some();

    if !is_comment && !is_issue {
        return;
    }

    // 2. Determine recipients and message type
    let mut recipients = Vec::new();
    let mut thread_id = resource_id.to_string();
    let mut subject = String::new();
    let mut content_prefix = String::new();

    if is_issue {
        // Get current involved
        let new_involved: Vec<String> = resource
            .get("involved")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if let Some(old) = old_resource {
            // Update: Check for newly added users
            let old_involved: Vec<String> = old
                .get("involved")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            // Find users in new but not in old
            for user in new_involved {
                if !old_involved.contains(&user) {
                    recipients.push(user);
                }
            }

            if recipients.is_empty() {
                return; // No new users added, no notification needed for issue update
            }

            subject = format!(
                "Je bent toegevoegd aan Zaak: {}",
                resource
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Naamloos")
            );
            content_prefix = "Je bent toegevoegd aan deze zaak.".to_string();
        } else {
            // New Issue: Notify all involved
            recipients = new_involved;
            subject = format!(
                "Nieuwe Zaak: {}",
                resource
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Naamloos")
            );
        }
    } else if is_comment {
        // Only notify for NEW comments (old_resource is None)
        if old_resource.is_some() {
            return; // Skip edits
        }

        // For comments, the thread_id IS the event subject (which is the issue ID)
        // We trust the frontend/event creator to set this correctly.
        thread_id = event.subject.clone();

        // Fetch the parent issue to get involved users
        if let Ok(Some(parent)) = state.storage.get_resource(&thread_id).await {
            if let Some(involved) = parent.get("involved").and_then(|v| v.as_array()) {
                for user in involved {
                    if let Some(u) = user.as_str() {
                        recipients.push(u.to_string());
                    }
                }
            }
        }
        subject = format!("Nieuwe Reactie op {}", thread_id);
    }

    // 3. Determine author (to exclude from notifications)
    // Use the CloudEvent source as the author.
    let author = &event.source;

    // 4. Send emails
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| "https://zaakchat.nl".to_string());

    for recipient in recipients {
        // Skip author
        if recipient == author.as_str() {
            continue;
        }

        // Smart Suppression: Check if user is active (seen in last 2 mins)
        if let Some(last_seen) = state.active_users.get(&recipient) {
            if last_seen.elapsed() < Duration::from_secs(120) {
                println!("[notify] Suppressing email to {} (active)", recipient);
                continue;
            }
        }

        let content = if is_issue {
            resource
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
        } else {
            resource
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("")
        };

        let full_content = if !content_prefix.is_empty() {
            format!("{}\n\n{}", content_prefix, content)
        } else {
            content.to_string()
        };

        // Generate magic link token
        let magic_link = match crate::auth::create_jwt(&recipient) {
            Ok(token) => {
                let link = format!(
                    "{}/verify-login?token={}&redirect=/zaak/{}",
                    base_url, token, thread_id
                );
                println!("[notify] Generated magic link for {}: {}", recipient, link);
                link
            }
            Err(e) => {
                eprintln!("[notify] Failed to create JWT for {}: {}", recipient, e);
                format!("{}/zaak/{}", base_url, thread_id) // Fallback to normal link
            }
        };

        let html_body = format!(
            "<html><body><p>{}</p><p><a href=\"{}\">Bekijk in ZaakChat</a></p></body></html>",
            full_content.replace("\n", "<br>"),
            magic_link
        );
        let text_body = format!("{}\n\nBekijk in ZaakChat: {}", full_content, magic_link);

        // Reply-To: hash+issue_id@inbound.postmarkapp.com
        let reply_to = format!(
            "c677cf964ad4b602877125dc320323ab+{}@inbound.postmarkapp.com",
            thread_id
        );

        println!(
            "[notify] Sending email to {} for thread {}",
            recipient, thread_id
        );
        tokio::spawn({
            let email_service = state.email_service.clone();
            let recipient = recipient.clone();
            let subject = subject.clone();
            let html_body = html_body.clone();
            let text_body = text_body.clone();
            let reply_to = reply_to.clone();
            let thread_id = thread_id.clone();
            async move {
                if let Err(e) = email_service
                    .send_notification(
                        &recipient,
                        &subject,
                        &html_body,
                        &text_body,
                        Some(&reply_to),
                        Some(&thread_id),
                    )
                    .await
                {
                    eprintln!("[notify] Failed to send email to {}: {}", recipient, e);
                }
            }
        });
    }
}

/// Process an event and update resources accordingly
pub async fn process_event(
    state: &AppState,
    event: &CloudEvent,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Extract data from the event
    let data = match &event.data {
        Some(d) => d,
        None => return Ok(()), // No data to process
    };

    // Check if this is a JSONCommit event (accept both legacy and NL-VNG names)
    if event.event_type == "nl.vng.zaken.json-commit.v1" || event.event_type == "json.commit" {
        let commit: JSONCommit = serde_json::from_value(data.clone())?;

        // Handle deletion
        if commit.deleted.unwrap_or(false) {
            state.storage.delete_resource(&commit.resource_id).await?;
            return Ok(());
        }

        // Determine resource type more robustly:
        let mut resource_type = extract_resource_type_from_schema(&commit.schema).to_string();

        if resource_type == "unknown" {
            let subj_type = extract_resource_type_from_subject(&event.subject);
            if subj_type != "unknown" {
                resource_type = subj_type.to_string();
            }
        }

        if resource_type == "unknown" {
            if let Some(resource_data) = &commit.resource_data {
                if resource_data.is_object() {
                    let obj = resource_data.as_object().unwrap();
                    if obj.contains_key("title") {
                        resource_type = "Issue".to_string();
                    } else if obj.contains_key("content") {
                        resource_type = "Comment".to_string();
                    } else if obj.contains_key("cta") {
                        resource_type = "Task".to_string();
                    } else if obj.contains_key("moments") {
                        resource_type = "Planning".to_string();
                    } else if obj.get("url").is_some() || obj.get("size").is_some() {
                        resource_type = "Document".to_string();
                    }
                }
            }
        }

        // Get existing resource if it exists
        let existing_resource = state.storage.get_resource(&commit.resource_id).await?;
        let old_resource = existing_resource.clone(); // Capture old state

        // Apply changes (merge patch or replace with resource_data)
        let new_resource = if let Some(mut existing) = existing_resource {
            // Apply patch if provided
            if let Some(patch) = &commit.patch {
                apply_json_merge_patch(&mut existing, patch);
            }
            // Override with full resource_data if provided
            if let Some(resource_data) = &commit.resource_data {
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
        state
            .storage
            .store_resource(&commit.resource_id, &resource_type, &new_resource)
            .await?;

        // Schedule background indexing of the resource via the search subsystem.
        let resource_id = commit.resource_id.clone();
        let resource_type_clone = resource_type.clone();
        let mut data_clone = new_resource.clone();
        let search = state.search.clone();
        // AUTH FIX: Denormalize 'involved' for Comments (and other child resources)
        // Comments don't have 'involved' field, so they fail the default auth filter.
        // We look up the parent issue and copy its 'involved' list into the indexing payload.
        if (resource_type_clone == "Comment" || resource_type_clone == "comment")
            && data_clone.get("involved").is_none()
        {
            // Use event.subject as the parent Issue ID
            // The frontend sends zaakId as subject for Comments
            let parent_id = event.subject.clone();

            if let Ok(Some(parent)) = state.storage.get_resource(&parent_id).await {
                if let Some(involved) = parent.get("involved") {
                    if let Some(obj) = data_clone.as_object_mut() {
                        obj.insert("involved".to_string(), involved.clone());
                    }
                }
            }
        }

        let timestamp_opt = commit
            .timestamp
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let payload = serde_json::to_string(&data_clone).unwrap_or_default();

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
                "[handlers] failed adding resource payload to search index id={} err={}",
                resource_id, err
            );
        }

        // Trigger Notifications
        send_notifications_for_event(state, event, &new_resource, old_resource.as_ref()).await;
    } else {
        // For other event types, we'll just store them as-is
        let resource_type = extract_resource_type_from_subject(&event.subject);
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
        // Index resource synchronously
        if let Err(err) = search
            .add_resource_payload(&id_clone, &rt_clone, "", &payload, None)
            .await
        {
            eprintln!(
                "[handlers] failed adding non-json-commit resource payload id={} err={}",
                id_clone, err
            );
        }
    }

    Ok(())
}

/// Extract resource type from schema URL
fn extract_resource_type_from_schema(schema: &str) -> &str {
    if schema.contains("Issue") {
        "Issue"
    } else if schema.contains("Comment") {
        "Comment"
    } else if schema.contains("Task") {
        "Task"
    } else if schema.contains("Planning") {
        "Planning"
    } else if schema.contains("Document") {
        "Document"
    } else {
        "unknown"
    }
}

/// Extract resource type from subject
fn extract_resource_type_from_subject(subject: &str) -> &str {
    if subject.contains("issue") {
        "Issue"
    } else if subject.contains("comment") {
        "Comment"
    } else if subject.contains("task") {
        "Task"
    } else if subject.contains("planning") {
        "Planning"
    } else if subject.contains("document") {
        "Document"
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

use crate::auth::AuthUser;

/// GET /query - Search resources using full-text search
/// Returns structured search results produced by the storage layer.
pub async fn query_resources(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SearchResult>>, StatusCode> {
    // Always use the authenticated user for filtering
    let user = &auth_user.user_id;
    let final_query = crate::search::SearchIndex::apply_authorization_filter(&params.q, user);

    let results = state
        .search
        .search(&state.storage, &final_query, params.limit)
        .await
        .map_err(|e| {
            eprintln!("Failed to search resources: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(results))
}

/// POST /api/email/inbound - Handle incoming Postmark webhooks
pub async fn inbound_email_handler(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<StatusCode, StatusCode> {
    println!("[inbound] Received webhook");

    // 1. Extract Sender (From)
    let from = payload
        .get("From")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    // Extract email from "Name <email@domain.com>" format if needed
    // Simple extraction:
    let sender_email = if let Some(start) = from.find('<') {
        if let Some(end) = from.find('>') {
            &from[start + 1..end]
        } else {
            from
        }
    } else {
        from
    };

    // 2. Extract Thread ID (Issue ID) from OriginalRecipient
    // Format: c677cf964ad4b602877125dc320323ab+<issue_id>@inbound.postmarkapp.com
    let recipient = payload
        .get("OriginalRecipient")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;
    let parts: Vec<&str> = recipient.split('+').collect();
    if parts.len() < 2 {
        eprintln!("[inbound] Invalid recipient format: {}", recipient);
        return Err(StatusCode::BAD_REQUEST);
    }
    let issue_id_part = parts[1];
    let issue_id = issue_id_part.split('@').next().unwrap_or(issue_id_part);

    // 3. Extract Content (TextBody)
    // Postmark provides TextBody and HtmlBody. We prefer TextBody for comments.
    // We might need to strip the quoted reply (Postmark usually handles this via StrippedTextReply, but let's check)
    let content = payload
        .get("StrippedTextReply")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .or_else(|| payload.get("TextBody").and_then(|v| v.as_str()))
        .unwrap_or("");

    if content.is_empty() {
        eprintln!("[inbound] Empty content");
        return Ok(StatusCode::OK); // Don't error, just ignore
    }

    println!(
        "[inbound] Parsed reply from {} for issue {}: {}",
        sender_email, issue_id, content
    );

    // 4. Create Comment
    let comment_id = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();

    let comment_data = serde_json::json!({
        "id": comment_id,
        "parent_id": issue_id,
        "content": content,
        "author": sender_email,
        "created_at": timestamp,
    });

    let event = CloudEvent {
        specversion: "1.0".to_string(),
        id: uuid::Uuid::new_v4().to_string(),
        // Use sender email as source so they are identified as author
        source: sender_email.to_string(),
        // Subject should be the Issue ID (thread ID)
        subject: issue_id.to_string(),
        event_type: "json.commit".to_string(),
        time: Some(timestamp.clone()),
        datacontenttype: Some("application/json".to_string()),
        dataschema: None,
        dataref: None,
        sequence: None,
        sequencetype: None,
        data: Some(serde_json::json!({
            "resource_id": comment_id,
            "schema": "https://zaakchat.nl/schemas/Comment.json",
            "timestamp": timestamp,
            "resource_data": comment_data
        })),
    };

    // Use handle_event logic (store, index, broadcast)
    // We can't call handle_event directly because of Axum types, so we replicate the logic or extract a shared function.
    // For simplicity, let's call the internal logic.

    let seq_key = state.storage.store_event(&event).await.map_err(|e| {
        eprintln!("Failed to store inbound event: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // We need to mutate event to add sequence, but we can't easily here without cloning.
    // Let's just create a new event with sequence for broadcasting.
    let mut broadcast_event = event.clone();
    broadcast_event.sequence = Some(seq_key);

    // Indexing
    {
        let search = state.search.clone();
        let payload = serde_json::to_string(&broadcast_event).unwrap_or_default();
        let id = broadcast_event.id.clone();
        let doc_type = broadcast_event.event_type.clone();
        if let Err(e) = search
            .add_event_payload(&id, &doc_type, "", &payload, None)
            .await
        {
            eprintln!("[inbound] failed indexing: {}", e);
        }
    }

    // Process (store resource)
    if let Err(e) = process_event(&state, &broadcast_event).await {
        eprintln!("[inbound] failed processing: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Commit search
    let _ = state.search.commit().await;

    // Broadcast
    let _ = state.tx.send(broadcast_event);

    Ok(StatusCode::OK)
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
            extract_resource_type_from_schema("https://zaakchat.nl/schemas/Issue.json"),
            "Issue"
        );
        assert_eq!(
            extract_resource_type_from_schema("https://zaakchat.nl/schemas/Comment.json"),
            "Comment"
        );
        assert_eq!(
            extract_resource_type_from_schema("https://other.com/schemas/Task"),
            "Task"
        );
        assert_eq!(extract_resource_type_from_schema("unknown"), "unknown");
    }

    #[test]
    fn test_extract_resource_type_from_subject() {
        assert_eq!(
            extract_resource_type_from_subject("new issue created"),
            "Issue"
        );
        assert_eq!(
            extract_resource_type_from_subject("comment added"),
            "Comment"
        );
        assert_eq!(extract_resource_type_from_subject("unknown"), "unknown");
    }

    #[tokio::test]
    async fn test_integration_event_processing_and_search(
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use crate::email::EmailService;
        use crate::handlers::{handle_event, AppState};
        use crate::search::SearchIndex;
        use crate::storage::Storage;
        use chrono::Utc;
        use std::sync::Arc;
        use tempfile::TempDir;
        use tokio::sync::broadcast;

        let dir = TempDir::new()?;
        let storage_path = dir.path().join("data");
        std::fs::create_dir_all(&storage_path)?;
        let index_path = dir.path().join("index");
        // SearchIndex creates dir if missing

        let storage = Arc::new(Storage::new(&storage_path).await?);
        let search = Arc::new(SearchIndex::open(
            &index_path,
            true,
            std::time::Duration::from_millis(50),
        )?); // fast commit
        let (tx, _rx) = broadcast::channel(100);

        let transport = Arc::new(crate::email::MockTransport::new(
            "http://test.local".to_string(),
        ));
        let email_service = Arc::new(EmailService::new(transport));

        // Use AppState::new to correctly initialize all fields (active_users, push_subscriptions)
        let state = AppState::new(storage, search, tx, email_service);

        use axum::extract::State;
        use axum::Json;

        // Define test user
        let user = "integration@example.com";

        // 1. Create Issue Event
        let issue_id = "issue-int-1";
        let issue_event = crate::schemas::CloudEvent {
            id: "evt-1".to_string(),
            source: "test".to_string(),
            specversion: "1.0".to_string(),
            event_type: "json.commit".to_string(),
            subject: issue_id.to_string(),
            time: Some(Utc::now().to_rfc3339()),
            datacontenttype: Some("application/json".to_string()),
            dataschema: None,
            dataref: None,
            sequencetype: None,
            data: Some(serde_json::json!({
                "resource_id": issue_id,
                "schema": "https://zaakchat.nl/schemas/Issue.json",
                "resource_data": {
                    "title": "Integration Issue",
                    "status": "open",
                    "involved": [user]
                },
                "msg_type": "resource",
                "commit_id": "c1",
                "author": "me",
                "timestamp": Utc::now().to_rfc3339()
            })),
            sequence: None,
        };

        handle_event(State(state.clone()), Json(issue_event))
            .await
            .unwrap();

        // 2. Create Comment Event (referencing Issue)
        let comment_id = "comment-int-1";
        let comment_event = crate::schemas::CloudEvent {
            id: "evt-2".to_string(),
            source: "test".to_string(),
            specversion: "1.0".to_string(),
            event_type: "json.commit".to_string(),
            subject: issue_id.to_string(),
            time: Some(Utc::now().to_rfc3339()),
            datacontenttype: Some("application/json".to_string()),
            dataschema: None,
            dataref: None,
            sequencetype: None,
            data: Some(serde_json::json!({
                "resource_id": comment_id,
                "schema": "https://zaakchat.nl/schemas/Comment.json",
                "resource_data": {
                    "content": "Integration Comment",
                    "quote_comment": null
                },
                 "msg_type": "resource",
                 "commit_id": "c2",
                 "author": "me",
                 "timestamp": Utc::now().to_rfc3339()
            })),
            sequence: None,
        };

        // Inject subject (Issue ID) so process_event knows the parent
        let mut comment_event = comment_event;
        comment_event.subject = issue_id.to_string();

        handle_event(State(state.clone()), Json(comment_event))
            .await
            .unwrap();

        // Allow indexing (handle_event calls commit, but let's be safe or wait if needed)
        // handle_event calls search.commit() at the end, so it should be visible.

        // 3. Search
        let q_auth = SearchIndex::apply_authorization_filter("type:Comment", user);
        let results = state
            .search
            .search_best_effort(&state.storage, &q_auth, 10)
            .await;

        let found = results.iter().any(|r| r.id == comment_id);

        if !found {
            println!(
                "DEBUG: Authorized search returned {} results.",
                results.len()
            );
            for r in &results {
                println!("Result: {:?}", r);
            }
        }

        assert!(
            found,
            "Should find Comment with injected involved field via handle_event pipeline"
        );

        Ok(())
    }
}

/// Login Request
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
}

/// Login Response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
}

/// POST /login - Initiate passwordless login
pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Generate a short-lived JWT (15 minutes) for the magic link
    let token =
        match crate::auth::create_jwt_with_expiry(&payload.email, chrono::Duration::minutes(15)) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Failed to create login JWT: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

    // Send magic link
    if let Err(e) = state
        .email_service
        .send_magic_link(&payload.email, &token)
        .await
    {
        eprintln!("Failed to send magic link: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(serde_json::json!({
        "message": "Magic link sent. Check your email."
    })))
}

/// GET /auth/verify - Verify magic link token
#[derive(Deserialize)]
pub struct VerifyParams {
    token: String,
}

pub async fn verify_login_handler(
    State(_state): State<AppState>,
    Query(params): Query<VerifyParams>,
) -> Result<Json<LoginResponse>, StatusCode> {
    // Verify the token directly as a JWT
    match crate::auth::verify_jwt(&params.token) {
        Ok(claims) => {
            // Token is valid. Issue a new long-lived session JWT (24h).
            match crate::auth::create_jwt(&claims.sub) {
                Ok(token) => Ok(Json(LoginResponse { token })),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Err(_) => {
            // Invalid or expired
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

#[cfg(test)]
mod tests_access {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_check_access_with_hyphenated_email() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::new(temp_dir.path()).await.unwrap();

        let issue_id = "issue-1";
        let user_id = "test-user@example.com";

        let issue_data = serde_json::json!({
            "title": "Test Issue",
            "involved": [user_id]
        });

        storage
            .store_resource(issue_id, "issue", &issue_data)
            .await
            .unwrap();

        let has_access = check_access(&storage, user_id, issue_id).await;
        assert!(has_access, "User should have access to the issue");

        let other_user = "other@example.com";
        let has_access_other = check_access(&storage, other_user, issue_id).await;
        assert!(!has_access_other, "Other user should NOT have access");
    }
}

/// Reset handler for E2E tests
pub async fn reset_handler(
    State(state): State<AppState>,
) -> Result<impl axum::response::IntoResponse, (axum::http::StatusCode, String)> {
    // 1. Clear storage
    if let Err(e) = state.storage.clear().await {
        return Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to clear storage: {}", e),
        ));
    }

    // 2. Clear search index
    if let Err(e) = state.search.clear().await {
        return Err((
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to clear search index: {}", e),
        ));
    }

    // 3. Clear active users
    state.active_users.clear();

    println!("[reset] Server state wiped (storage + search + active_users)");

    Ok(axum::http::StatusCode::OK)
}
