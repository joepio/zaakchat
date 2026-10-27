//! JSON Commit transactions on `POST /events`.
//!
//! A `json.commit` event carries one JSONCommit; a `json.commit.transaction` event carries
//! `data.commits`, a list of JSONCommits. Either way the server validates and authorises
//! every commit first, then stores the events and all resource changes in one storage
//! transaction: all or nothing.
//!
//! The `subject` is the aggregate every commit must belong to: a new resource joins it,
//! an existing one must already be in it. The idempotency key is the caller plus the
//! CloudEvent `source` + `id`, so a replay never returns another caller's result.
//!
//! Follows the JSON Commit pattern in VNG-API-Lab (`patterns/json-commit/next.yaml`).

use std::collections::HashMap;

use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::{json, Value};

use crate::handlers::{
    apply_json_merge_patch, extract_resource_type_from_schema, extract_resource_type_from_subject,
    index_and_notify, AppState,
};
use crate::schemas::{CloudEvent, JSONCommit};
use crate::storage::{IdempotencyRecord, ResourceWrite, TransactionError};

pub const TYPE_COMMIT: &str = "json.commit";
/// Older name for `json.commit`, still accepted on input.
pub const TYPE_COMMIT_LEGACY: &str = "nl.vng.zaken.json-commit.v1";
pub const TYPE_TRANSACTION: &str = "json.commit.transaction";

/// Upper bound on commits in one transaction.
pub const MAX_COMMITS: usize = 100;

pub fn is_commit_event(event_type: &str) -> bool {
    matches!(
        event_type,
        TYPE_COMMIT | TYPE_COMMIT_LEGACY | TYPE_TRANSACTION
    )
}

/// One problem with one part of the request (`invalidParams` in problem+json).
#[derive(Debug, Clone, Serialize)]
pub struct InvalidParam {
    pub name: String,
    pub code: String,
    pub reason: String,
}

impl InvalidParam {
    fn new(name: impl Into<String>, code: &str, reason: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            code: code.to_string(),
            reason: reason.into(),
        }
    }
}

fn problem(status: StatusCode, title: &str, detail: &str, params: &[InvalidParam]) -> Response {
    let mut body = json!({
        "type": "about:blank",
        "title": title,
        "status": status.as_u16(),
        "detail": detail,
    });
    if !params.is_empty() {
        body["invalidParams"] = json!(params);
    }
    (
        status,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        )],
        body.to_string(),
    )
        .into_response()
}

fn json_response(status: StatusCode, body: String) -> Response {
    (
        status,
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        )],
        body,
    )
        .into_response()
}

/// Answer for a request whose idempotency key was already used.
fn replay_or_conflict(rec: IdempotencyRecord, body: &str) -> Response {
    if rec.body == body {
        let mut res = json_response(StatusCode::CREATED, rec.response);
        res.headers_mut()
            .insert("Idempotent-Replayed", HeaderValue::from_static("true"));
        res
    } else {
        problem(
            StatusCode::CONFLICT,
            "Event id already used",
            "An event with this source and id was already processed with a different body. Use a new id.",
            &[],
        )
    }
}

/// Resource type from the commit's schema, the event subject, or the resource's fields.
fn resource_type_for(commit: &JSONCommit, subject: &str) -> String {
    let from_schema = extract_resource_type_from_schema(&commit.schema);
    if from_schema != "unknown" {
        return from_schema.to_string();
    }
    let from_subject = extract_resource_type_from_subject(subject);
    if from_subject != "unknown" {
        return from_subject.to_string();
    }
    if let Some(obj) = commit.resource_data.as_ref().and_then(|v| v.as_object()) {
        if obj.contains_key("title") {
            return "Issue".to_string();
        } else if obj.contains_key("content") {
            return "Comment".to_string();
        } else if obj.contains_key("cta") {
            return "Task".to_string();
        } else if obj.contains_key("moments") {
            return "Planning".to_string();
        } else if obj.get("url").is_some() || obj.get("size").is_some() {
            return "Document".to_string();
        }
    }
    "unknown".to_string()
}

fn is_involved(resource: &Value, user: &str) -> Option<bool> {
    resource
        .get("involved")
        .and_then(|v| v.as_array())
        .map(|list| list.iter().any(|p| p.as_str() == Some(user)))
}

/// Resource state as seen inside a transaction: earlier commits in the same
/// transaction win over what is stored.
struct Overlay<'a> {
    state: &'a AppState,
    /// `None` = deleted earlier in this transaction.
    changed: HashMap<String, Option<Value>>,
}

impl Overlay<'_> {
    async fn get(&self, id: &str) -> Result<Option<Value>, StatusCode> {
        if let Some(v) = self.changed.get(id) {
            return Ok(v.clone());
        }
        self.state
            .storage
            .get_resource(id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// Validate, authorise and apply a `json.commit` or `json.commit.transaction` event.
///
/// `user` is the logged-in user: it becomes the `actor` of every commit and is checked
/// against the `involved` list of the aggregate (`subject`) or the resource itself.
/// `None` is for trusted internal callers such as inbound email, which set `actor` themselves.
pub async fn handle_commit_event(
    state: &AppState,
    user: Option<&str>,
    event: CloudEvent,
) -> Result<Response, StatusCode> {
    let body = serde_json::to_string(&event).map_err(|_| StatusCode::BAD_REQUEST)?;
    // Bound to the caller: a stored response holds resourceData, so another caller that
    // reuses the same `source` + `id` must not get it back.
    let idempotency_key = format!(
        "{}\n{}\n{}",
        user.unwrap_or("system"),
        event.source,
        event.id
    );

    if let Some(rec) = state
        .storage
        .get_idempotency(&idempotency_key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return Ok(replay_or_conflict(rec, &body));
    }

    let is_transaction = event.event_type == TYPE_TRANSACTION;

    // 1. Collect the raw commits with the JSON pointer each one lives at.
    let raw: Vec<(String, Value)> = if is_transaction {
        let commits = event
            .data
            .as_ref()
            .and_then(|d| d.get("commits"))
            .and_then(|c| c.as_array());
        match commits {
            Some(list) if !list.is_empty() && list.len() <= MAX_COMMITS => list
                .iter()
                .enumerate()
                .map(|(i, c)| (format!("/data/commits/{}", i), c.clone()))
                .collect(),
            Some(list) if list.len() > MAX_COMMITS => {
                return Ok(problem(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Transaction rejected, nothing stored",
                    "Too many commits in one transaction.",
                    &[InvalidParam::new(
                        "/data/commits",
                        "too-many",
                        format!("At most {} commits per transaction.", MAX_COMMITS),
                    )],
                ))
            }
            _ => {
                return Ok(problem(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Transaction rejected, nothing stored",
                    "A json.commit.transaction needs a non-empty data.commits list.",
                    &[InvalidParam::new(
                        "/data/commits",
                        "required",
                        "data.commits must be a non-empty list of JSONCommits.",
                    )],
                ))
            }
        }
    } else {
        match &event.data {
            Some(d) if d.is_object() => vec![("/data".to_string(), d.clone())],
            _ => {
                return Ok(problem(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "Commit rejected, nothing stored",
                    "A json.commit event needs a JSONCommit object in data.",
                    &[InvalidParam::new(
                        "/data",
                        "required",
                        "data must be a JSONCommit object.",
                    )],
                ))
            }
        }
    };

    // 2. Parse and check the shape of every commit before touching anything.
    let mut errors: Vec<InvalidParam> = Vec::new();
    let mut commits: Vec<(String, JSONCommit)> = Vec::new();
    for (pointer, value) in raw {
        match serde_json::from_value::<JSONCommit>(value) {
            Ok(commit) => {
                if commit.resource_id.trim().is_empty() {
                    errors.push(InvalidParam::new(
                        format!("{}/resourceId", pointer),
                        "required",
                        "resourceId must not be empty.",
                    ));
                }
                let ops = [
                    commit.resource_data.is_some(),
                    commit.patch.is_some(),
                    commit.deleted == Some(true),
                ]
                .iter()
                .filter(|b| **b)
                .count();
                if ops != 1 {
                    errors.push(InvalidParam::new(
                        pointer.clone(),
                        "one-operation",
                        "A commit needs exactly one of resourceData, patch or deleted: true.",
                    ));
                }
                commits.push((pointer, commit));
            }
            Err(e) => errors.push(InvalidParam::new(pointer, "invalid", e.to_string())),
        }
    }
    if !errors.is_empty() {
        return Ok(problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Transaction rejected, nothing stored",
            "One or more commits are invalid.",
            &errors,
        ));
    }

    // 3. Work out the end state in memory, in order, authorising each commit against
    //    the state left by the commits before it.
    let mut overlay = Overlay {
        state,
        changed: HashMap::new(),
    };
    let mut forbidden = false;
    let mut stale: Vec<InvalidParam> = Vec::new();
    // Last commit per resource as changed by this transaction (`None` = deleted).
    let mut heads: HashMap<String, Option<String>> = HashMap::new();
    let commit_event_id = |i: usize| {
        if is_transaction {
            format!("{}:{}", event.id, i)
        } else {
            event.id.clone()
        }
    };
    let mut writes: Vec<ResourceWrite> = Vec::new();
    // (commit, resource type, new state, old state) per commit, for after the commit.
    let mut applied: Vec<(JSONCommit, String, Option<Value>, Option<Value>)> = Vec::new();
    let now = chrono::Utc::now().to_rfc3339();

    for (pointer, mut commit) in commits {
        let current = overlay.get(&commit.resource_id).await?;

        // The commit must belong to the subject. Resources created earlier in this
        // transaction joined the subject; stored ones keep the aggregate they were created in.
        if current.is_some() && !overlay.changed.contains_key(&commit.resource_id) {
            let home = state
                .storage
                .get_aggregate(&commit.resource_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .unwrap_or_else(|| commit.resource_id.clone());
            if home != event.subject {
                // Only tell callers who may see the other aggregate that the resource is there.
                let may_see = match user {
                    None => true,
                    Some(user) => overlay
                        .get(&home)
                        .await?
                        .and_then(|a| is_involved(&a, user))
                        .unwrap_or(false),
                };
                if may_see {
                    errors.push(InvalidParam::new(
                        format!("{}/resourceId", pointer),
                        "subject-mismatch",
                        format!(
                            "{} belongs to {}, not to subject {}.",
                            commit.resource_id, home, event.subject
                        ),
                    ));
                } else {
                    forbidden = true;
                }
                continue;
            }
        }

        let aggregate = overlay.get(&event.subject).await?;
        if aggregate.is_none() && commit.resource_id != event.subject {
            errors.push(InvalidParam::new(
                "/subject",
                "not-found",
                format!(
                    "Subject {} does not exist; create it first, in this or an earlier transaction.",
                    event.subject
                ),
            ));
            continue;
        }

        if let Some(user) = user {
            // Write access follows the aggregate's `involved` list. Creating the aggregate
            // itself is allowed; an aggregate without such a list is open (demo data).
            let allowed = aggregate
                .as_ref()
                .and_then(|a| is_involved(a, user))
                .unwrap_or(true);
            if !allowed {
                forbidden = true;
                continue;
            }
            commit.actor = user.to_string();
        }
        if commit.timestamp.is_none() {
            commit.timestamp = Some(now.clone());
        }

        // Optimistic concurrency: the client says which commit it last saw on this resource.
        if let Some(expected) = &commit.previous_commit {
            let head = match heads.get(&commit.resource_id) {
                Some(h) => h.clone(),
                None => state
                    .storage
                    .get_head(&commit.resource_id)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            };
            if head.as_deref() != Some(expected.as_str()) {
                stale.push(InvalidParam::new(
                    format!("{}/previousCommit", pointer),
                    "stale",
                    format!(
                        "{} has changed since {}; the last commit is {}.",
                        commit.resource_id,
                        expected,
                        head.as_deref().unwrap_or("none")
                    ),
                ));
                continue;
            }
        }
        let commit_id = commit_event_id(applied.len());

        let resource_type = resource_type_for(&commit, &event.subject);
        let new_state = if commit.deleted == Some(true) {
            if current.is_none() {
                errors.push(InvalidParam::new(
                    format!("{}/resourceId", pointer),
                    "not-found",
                    format!("Resource {} does not exist.", commit.resource_id),
                ));
                continue;
            }
            writes.push(ResourceWrite::Delete {
                id: commit.resource_id.clone(),
            });
            heads.insert(commit.resource_id.clone(), None);
            None
        } else {
            let data = if let Some(patch) = &commit.patch {
                let Some(mut existing) = current.clone() else {
                    errors.push(InvalidParam::new(
                        format!("{}/resourceId", pointer),
                        "not-found",
                        format!(
                            "Resource {} does not exist; use resourceData to create it.",
                            commit.resource_id
                        ),
                    ));
                    continue;
                };
                apply_json_merge_patch(&mut existing, patch);
                existing
            } else {
                commit.resource_data.clone().unwrap_or_else(|| json!({}))
            };
            writes.push(ResourceWrite::Upsert {
                id: commit.resource_id.clone(),
                resource_type: resource_type.clone(),
                data: data.clone(),
                aggregate: event.subject.clone(),
                commit_id: commit_id.clone(),
            });
            heads.insert(commit.resource_id.clone(), Some(commit_id));
            Some(data)
        };

        overlay
            .changed
            .insert(commit.resource_id.clone(), new_state.clone());
        applied.push((commit, resource_type, new_state, current));
    }

    // One decision for the whole transaction, without per-commit details that could
    // reveal what exists in aggregates the caller may not see.
    if forbidden {
        return Ok(problem(
            StatusCode::FORBIDDEN,
            "Transaction rejected, nothing stored",
            "You are not allowed to make these changes.",
            &[],
        ));
    }
    if !stale.is_empty() {
        return Ok(problem(
            StatusCode::PRECONDITION_FAILED,
            "Transaction rejected, nothing stored",
            "A resource changed since the commit named in previousCommit. Fetch it again and retry.",
            &stale,
        ));
    }
    if !errors.is_empty() {
        return Ok(problem(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Transaction rejected, nothing stored",
            "One or more commits cannot be applied.",
            &errors,
        ));
    }

    // 4. One event per commit, so consumers that read one commit per event keep working.
    let transactionid = is_transaction.then(|| event.id.clone());
    let events: Vec<CloudEvent> = applied
        .iter()
        .enumerate()
        .map(|(i, (commit, _, _, _))| CloudEvent {
            specversion: "1.0".to_string(),
            id: commit_event_id(i),
            source: event.source.clone(),
            subject: event.subject.clone(),
            event_type: TYPE_COMMIT.to_string(),
            time: Some(event.time.clone().unwrap_or_else(|| now.clone())),
            datacontenttype: Some("application/json".to_string()),
            dataschema: None,
            dataref: None,
            sequence: None,
            sequencetype: None,
            transactionid: transactionid.clone(),
            data: serde_json::to_value(commit).ok(),
        })
        .collect();

    // 5. Store everything in one storage transaction.
    let txid = event.id.clone();
    let result = state
        .storage
        .apply_transaction(events, &writes, &idempotency_key, &body, |stored| {
            if is_transaction {
                json!({ "transactionId": txid, "events": stored }).to_string()
            } else {
                serde_json::to_string(&stored[0]).unwrap_or_default()
            }
        })
        .await;

    let (events, response) = match result {
        Ok(ok) => ok,
        Err(TransactionError::AlreadyProcessed(rec)) => return Ok(replay_or_conflict(rec, &body)),
        Err(TransactionError::Storage(e)) => {
            eprintln!("[commits] failed to store transaction {}: {}", event.id, e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // 6. Derived work after the commit: search index, notifications, live stream.
    //    These may lag or fail without leaving the stored state half-done.
    for (stored, (commit, resource_type, new_state, old_state)) in events.iter().zip(applied.iter())
    {
        let payload = serde_json::to_string(stored).unwrap_or_default();
        if let Err(e) = state
            .search
            .add_event_payload(&stored.id, "Event", "", &payload, None)
            .await
        {
            eprintln!("[commits] failed indexing event {}: {}", stored.id, e);
        }
        if let Some(new_state) = new_state {
            index_and_notify(
                state,
                stored,
                &commit.resource_id,
                resource_type,
                new_state,
                old_state.as_ref(),
                commit.timestamp.as_deref(),
            )
            .await;
        }
    }
    if let Err(e) = state.search.commit().await {
        eprintln!("[commits] failed to commit search index: {}", e);
    }
    for stored in &events {
        let _ = state.tx.send(stored.clone());
    }

    Ok(json_response(StatusCode::CREATED, response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::email::{EmailService, MockTransport};
    use crate::search::SearchIndex;
    use crate::storage::Storage;
    use axum::body::to_bytes;
    use std::sync::Arc;
    use tempfile::TempDir;

    const ALICE: &str = "alice@example.com";
    const MALLORY: &str = "mallory@example.com";

    async fn test_state(dir: &TempDir) -> AppState {
        let storage_path = dir.path().join("data");
        let storage = Arc::new(Storage::new(&storage_path).await.unwrap());
        let search = Arc::new(
            SearchIndex::open(
                dir.path().join("index"),
                true,
                std::time::Duration::from_millis(50),
            )
            .unwrap(),
        );
        let (tx, _rx) = tokio::sync::broadcast::channel(100);
        let email = Arc::new(EmailService::new(Arc::new(MockTransport::new(
            "http://test.local".to_string(),
        ))));
        AppState::new(storage, search, tx, email)
    }

    fn event(id: &str, event_type: &str, subject: &str, data: Value) -> CloudEvent {
        CloudEvent {
            specversion: "1.0".to_string(),
            id: id.to_string(),
            source: "test".to_string(),
            subject: subject.to_string(),
            event_type: event_type.to_string(),
            time: None,
            datacontenttype: Some("application/json".to_string()),
            dataschema: None,
            dataref: None,
            sequence: None,
            sequencetype: None,
            transactionid: None,
            data: Some(data),
        }
    }

    fn create_issue_and_comment(id: &str) -> CloudEvent {
        event(
            id,
            TYPE_TRANSACTION,
            "issue-1",
            json!({ "commits": [
                { "resourceId": "issue-1", "schema": "Issue",
                  "resourceData": { "title": "Paspoort", "status": "open", "involved": [ALICE] } },
                { "resourceId": "comment-1", "schema": "Comment",
                  "resourceData": { "content": "Wanneer kan ik hem ophalen?" } }
            ]}),
        )
    }

    async fn body_json(res: Response) -> Value {
        let bytes = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn transaction_applies_all_commits_with_one_transactionid() {
        let dir = TempDir::new().unwrap();
        let state = test_state(&dir).await;

        let res = handle_commit_event(&state, Some(ALICE), create_issue_and_comment("tx-1"))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        let body = body_json(res).await;
        assert_eq!(body["transactionId"], "tx-1");
        let events = body["events"].as_array().unwrap();
        assert_eq!(events.len(), 2);
        for e in events {
            assert_eq!(e["type"], TYPE_COMMIT);
            assert_eq!(e["transactionid"], "tx-1");
            assert_eq!(e["subject"], "issue-1");
            assert_eq!(e["data"]["actor"], ALICE);
        }

        assert!(state
            .storage
            .get_resource("issue-1")
            .await
            .unwrap()
            .is_some());
        assert!(state
            .storage
            .get_resource("comment-1")
            .await
            .unwrap()
            .is_some());

        // The stored feed carries the transaction id as well
        let feed = state.storage.list_events_after(None, 10).await.unwrap();
        assert_eq!(feed.len(), 2);
        assert!(feed
            .iter()
            .all(|e| e.transactionid.as_deref() == Some("tx-1")));
    }

    #[tokio::test]
    async fn failing_commit_rolls_back_the_whole_transaction() {
        let dir = TempDir::new().unwrap();
        let state = test_state(&dir).await;

        let bad = event(
            "tx-bad",
            TYPE_TRANSACTION,
            "issue-1",
            json!({ "commits": [
                { "resourceId": "issue-1", "schema": "Issue",
                  "resourceData": { "title": "Paspoort", "involved": [ALICE] } },
                { "resourceId": "comment-404", "schema": "Comment",
                  "patch": { "content": "patch on something that does not exist" } }
            ]}),
        );
        let res = handle_commit_event(&state, Some(ALICE), bad).await.unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = body_json(res).await;
        assert_eq!(
            body["invalidParams"][0]["name"],
            "/data/commits/1/resourceId"
        );
        assert_eq!(body["invalidParams"][0]["code"], "not-found");

        // Nothing stored: not the issue from commit 0, and no events
        assert!(state
            .storage
            .get_resource("issue-1")
            .await
            .unwrap()
            .is_none());
        assert!(state
            .storage
            .list_events_after(None, 10)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn commit_needs_exactly_one_operation() {
        let dir = TempDir::new().unwrap();
        let state = test_state(&dir).await;

        let both = event(
            "c-both",
            TYPE_COMMIT,
            "issue-1",
            json!({ "resourceId": "issue-1", "schema": "Issue",
                    "resourceData": { "title": "x" }, "patch": { "title": "y" } }),
        );
        let res = handle_commit_event(&state, Some(ALICE), both)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body_json(res).await["invalidParams"][0]["name"], "/data");
    }

    #[tokio::test]
    async fn same_id_same_body_replays_and_different_body_conflicts() {
        let dir = TempDir::new().unwrap();
        let state = test_state(&dir).await;

        let first = handle_commit_event(&state, Some(ALICE), create_issue_and_comment("tx-1"))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::CREATED);
        let first_body = body_json(first).await;

        let replay = handle_commit_event(&state, Some(ALICE), create_issue_and_comment("tx-1"))
            .await
            .unwrap();
        assert_eq!(replay.status(), StatusCode::CREATED);
        assert_eq!(replay.headers()["Idempotent-Replayed"], "true");
        assert_eq!(body_json(replay).await, first_body);
        // Not applied twice
        assert_eq!(
            state
                .storage
                .list_events_after(None, 10)
                .await
                .unwrap()
                .len(),
            2
        );

        let mut changed = create_issue_and_comment("tx-1");
        changed.data.as_mut().unwrap()["commits"][1]["resourceData"]["content"] =
            json!("iets anders");
        let conflict = handle_commit_event(&state, Some(ALICE), changed)
            .await
            .unwrap();
        assert_eq!(conflict.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn outsider_cannot_change_or_join_an_issue() {
        let dir = TempDir::new().unwrap();
        let state = test_state(&dir).await;
        handle_commit_event(&state, Some(ALICE), create_issue_and_comment("tx-1"))
            .await
            .unwrap();

        // Adding yourself to `involved`
        let join = event(
            "c-join",
            TYPE_COMMIT,
            "issue-1",
            json!({ "resourceId": "issue-1", "schema": "Issue",
                    "patch": { "involved": [ALICE, MALLORY] } }),
        );
        let res = handle_commit_event(&state, Some(MALLORY), join)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        // Commenting on someone else's issue
        let comment = event(
            "c-comment",
            TYPE_COMMIT,
            "issue-1",
            json!({ "resourceId": "comment-2", "schema": "Comment",
                    "resourceData": { "content": "hoi" } }),
        );
        let res = handle_commit_event(&state, Some(MALLORY), comment)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);

        let issue = state
            .storage
            .get_resource("issue-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(issue["involved"], json!([ALICE]));
        assert!(state
            .storage
            .get_resource("comment-2")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn actor_is_taken_from_the_logged_in_user() {
        let dir = TempDir::new().unwrap();
        let state = test_state(&dir).await;

        let spoofed = event(
            "c-spoof",
            TYPE_COMMIT,
            "issue-9",
            json!({ "resource_id": "issue-9", "schema": "Issue", "actor": "burgemeester@example.com",
                    "resource_data": { "title": "x", "involved": [ALICE] } }),
        );
        let res = handle_commit_event(&state, Some(ALICE), spoofed)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::CREATED);
        assert_eq!(body_json(res).await["data"]["actor"], ALICE);
    }

    /// Mallory owns issue-2 and claims it as subject to change Alice's comment in issue-1.
    #[tokio::test]
    async fn commit_outside_the_subject_is_refused() {
        let dir = TempDir::new().unwrap();
        let state = test_state(&dir).await;
        handle_commit_event(&state, Some(ALICE), create_issue_and_comment("tx-1"))
            .await
            .unwrap();
        let own = event(
            "c-own",
            TYPE_COMMIT,
            "issue-2",
            json!({ "resourceId": "issue-2", "schema": "Issue",
                    "resourceData": { "title": "Van Mallory", "involved": [MALLORY] } }),
        );
        handle_commit_event(&state, Some(MALLORY), own)
            .await
            .unwrap();

        // Patch, and overwrite through resourceData: both refused without saying why.
        for (id, data) in [
            (
                "c-patch",
                json!({ "resourceId": "comment-1", "schema": "Comment", "patch": { "content": "gewijzigd" } }),
            ),
            (
                "c-overwrite",
                json!({ "resourceId": "comment-1", "schema": "Comment", "resourceData": { "content": "gewijzigd" } }),
            ),
        ] {
            let res = handle_commit_event(
                &state,
                Some(MALLORY),
                event(id, TYPE_COMMIT, "issue-2", data),
            )
            .await
            .unwrap();
            assert_eq!(res.status(), StatusCode::FORBIDDEN);
            assert!(body_json(res).await.get("invalidParams").is_none());
        }
        let comment = state
            .storage
            .get_resource("comment-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(comment["content"], "Wanneer kan ik hem ophalen?");

        // Someone involved in both aggregates gets a 422 that names the mismatch.
        let wrong_subject = event(
            "c-wrong",
            TYPE_COMMIT,
            "issue-1",
            json!({ "resourceId": "issue-2", "schema": "Issue", "patch": { "title": "x" } }),
        );
        let res = handle_commit_event(&state, Some(MALLORY), wrong_subject)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            body_json(res).await["invalidParams"][0]["code"],
            "subject-mismatch"
        );
    }

    #[tokio::test]
    async fn commit_under_a_missing_subject_is_refused() {
        let dir = TempDir::new().unwrap();
        let state = test_state(&dir).await;
        let orphan = event(
            "c-orphan",
            TYPE_COMMIT,
            "issue-404",
            json!({ "resourceId": "comment-9", "schema": "Comment", "resourceData": { "content": "hoi" } }),
        );
        let res = handle_commit_event(&state, Some(ALICE), orphan)
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body_json(res).await["invalidParams"][0]["name"], "/subject");
    }

    /// The idempotency key includes the caller, so reusing someone's `source` + `id`
    /// never returns their stored response.
    #[tokio::test]
    async fn replay_is_bound_to_the_caller() {
        let dir = TempDir::new().unwrap();
        let state = test_state(&dir).await;
        handle_commit_event(&state, Some(ALICE), create_issue_and_comment("tx-1"))
            .await
            .unwrap();

        let res = handle_commit_event(&state, Some(MALLORY), create_issue_and_comment("tx-1"))
            .await
            .unwrap();
        assert!(res.headers().get("Idempotent-Replayed").is_none());
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        assert!(!body_json(res).await.to_string().contains("Paspoort"));
    }

    #[tokio::test]
    async fn stale_previous_commit_is_refused() {
        let dir = TempDir::new().unwrap();
        let state = test_state(&dir).await;
        handle_commit_event(&state, Some(ALICE), create_issue_and_comment("tx-1"))
            .await
            .unwrap();
        let patch = |id: &str, prev: &str, status: &str| {
            event(
                id,
                TYPE_COMMIT,
                "issue-1",
                json!({ "resourceId": "issue-1", "schema": "Issue",
                        "previousCommit": prev, "patch": { "status": status } }),
            )
        };

        // Two people both saw tx-1:0; the first one wins, the second gets 412.
        let first = handle_commit_event(
            &state,
            Some(ALICE),
            patch("c-a", "tx-1:0", "in_behandeling"),
        )
        .await
        .unwrap();
        assert_eq!(first.status(), StatusCode::CREATED);
        let second = handle_commit_event(&state, Some(ALICE), patch("c-b", "tx-1:0", "afgerond"))
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::PRECONDITION_FAILED);
        assert_eq!(
            body_json(second).await["invalidParams"][0]["name"],
            "/data/previousCommit"
        );
        let issue = state
            .storage
            .get_resource("issue-1")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(issue["status"], "in_behandeling");

        // After fetching again, the retry names the new head and succeeds.
        let retry = handle_commit_event(&state, Some(ALICE), patch("c-c", "c-a", "afgerond"))
            .await
            .unwrap();
        assert_eq!(retry.status(), StatusCode::CREATED);
    }
}
