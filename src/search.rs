use std::{
    error::Error,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
    collections::BTreeMap,
};


use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::schema::OwnedValue;
use tantivy::{doc, Index, IndexWriter, ReloadPolicy, TantivyDocument};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

use crate::schemas::CloudEvent;
use crate::storage::{SearchResult, Storage};

/// SearchIndex manages the Tantivy index: initialization, background commits,
/// and add/delete/search operations.
///
/// Responsibilities:
/// - Create/open index in given directory
/// - Provide async methods to add documents for events/resources (adds only)
/// - Periodically commit the writer (background task)
/// - Provide a `search` method that returns structured `SearchResult` entries,
///   hydrating persisted events/resources from `Storage`
///
/// Note: this module intentionally depends on Tantivy; storage.rs does not.
pub struct SearchIndex {
    index: Arc<Index>,
    writer: Arc<RwLock<IndexWriter>>,
    id_field: Field,
    type_field: Field,
    json_field: Field,
    timestamp_field: Field,
    // Background commit task handle (optional)
    commit_task: Option<JoinHandle<()>>,
}

impl SearchIndex {
    /// Open or create an index in `index_dir`. If `spawn_committer` is true,
    /// start a background commit task that commits every `commit_interval` seconds.
    pub fn open<P: AsRef<Path>>(
        index_dir: P,
        spawn_committer: bool,
        commit_interval: Duration,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let index_path = index_dir.as_ref();
        // Ensure the index directory exists before attempting to open/create the Tantivy directory.
        // This prevents failures when the directory is missing and avoids trying to open a non-existent path.
        if !index_path.exists() {
            std::fs::create_dir_all(index_path)?;
        }
        let _dir = MmapDirectory::open(index_path)?;
        // Build schema
        let mut schema_builder = Schema::builder();
        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let type_field = schema_builder.add_text_field("type", STRING | STORED);
        // Stored JSON payload field: we store the serialized JSON here and index it as a text field as well
        // so that Tantivy can tokenize and search the JSON content. We also store the field for hydration.
        let json_options = JsonObjectOptions::from(TEXT | STORED);
        let json_field = schema_builder.add_json_field("json_payload", json_options);
        let timestamp_field = schema_builder.add_date_field("timestamp", INDEXED | STORED);
        let schema = schema_builder.build();

        // Ensure the index is created or opened.
        // If an on-disk index already exists in the directory we open it; otherwise create a new index.
        // This avoids destructive recreation and keeps existing index files unless schema migration is desired.
        let index = if index_path.exists() && index_path.read_dir()?.next().is_some() {
            // Open existing index on disk
            Index::open_in_dir(index_path)?
        } else {
            // Create a new index directory with the current schema
            Index::create_in_dir(index_path, schema.clone())?
        };

        let writer = index.writer(50_000_000)?; // 50 MB heap for writer

        let si = Self {
            index: Arc::new(index),
            writer: Arc::new(RwLock::new(writer)),
            id_field,
            type_field,
            json_field,
            timestamp_field,
            commit_task: None,
        };

        // Optionally spawn a periodic committer
        let commit_task = if spawn_committer {
            let writer_clone = si.writer.clone();
            let interval = commit_interval;
            Some(tokio::spawn(async move {
                loop {
                    tokio::time::sleep(interval).await;
                    let mut w = writer_clone.write().await;
                    if let Err(e) = w.commit() {
                        eprintln!("[search][committer] commit failed: {}", e);
                    }
                }
            }))
        } else {
            None
        };

        let mut si = si;
        si.commit_task = commit_task;

        Ok(si)
    }

    /// Add an event document to the index (non-blocking with respect to commit).
    /// New behavior: callers can either call this helper with a CloudEvent (legacy),
    /// which will be serialized into a payload string and delegated to the payload-based API,
    /// or call `add_event_payload` directly if they already have a serialized payload.
    pub async fn add_event_doc(
        &self,
        event: &CloudEvent,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Build textual content (snippet)
        let content = format!(
            "{} {} {} {}",
            event.event_type,
            event.source,
            event.subject.as_deref().unwrap_or(""),
            event
                .data
                .as_ref()
                .map(|v| v.to_string())
                .unwrap_or_default()
        );

        // Serialize CloudEvent to payload JSON once, so callers don't repeatedly clone big structures
        let payload = serde_json::to_string(event).unwrap_or_default();

        // Parse optional timestamp into DateTime<Utc>
        let ts = event
            .time
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        // Delegate to payload-based add function
        self.add_event_payload(&event.id, &event.event_type, &content, &payload, ts)
            .await
    }

    /// Add an event to the index using already-serialized JSON payload.
    /// This avoids cloning/parsing heavy CloudEvent values in the caller.
    pub async fn add_event_payload(
        &self,
        id: &str,
        doc_type: &str,
        _content: &str,
        payload_json: &str,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let writer = self.writer.write().await;
        // Ensure we delete any existing document with this ID to prevent duplicates
        writer.delete_term(Term::from_field_text(self.id_field, id));

        // Build document and also populate structured fields where possible.
        // payload_json is already serialized by caller.
        let mut doc = doc!(
            self.id_field => id,
            self.type_field => doc_type,
        );

        // Parse JSON and add as JSON object
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(payload_json) {
            if let Some(obj) = json_val.as_object() {
                let tantivy_obj: BTreeMap<String, OwnedValue> = obj.iter()
                    .map(|(k, v)| (k.clone(), json_to_owned_value(v)))
                    .collect();
                doc.add_object(self.json_field, tantivy_obj);
            }
        }

        // No per-field indexing here: the full JSON payload is already indexed in `json_field`.
        // This keeps indexing simpler and avoids duplicating field extraction logic.
        // If needed, advanced JSON-aware queries can be used on `json_field`.

        if let Some(ts) = timestamp {
            doc.add_date(
                self.timestamp_field,
                tantivy::DateTime::from_timestamp_secs(ts.timestamp()),
            );
        } else {
            let now = Utc::now();
            doc.add_date(
                self.timestamp_field,
                tantivy::DateTime::from_timestamp_secs(now.timestamp()),
            );
        }

        writer.add_document(doc)?;
        // commit deferred to periodic committer
        Ok(())
    }

    /// Add a resource document to the index (non-blocking).
    /// New API: callers can provide the serialized payload string to avoid extra cloning.
    pub async fn add_resource_doc(
        &self,
        id: &str,
        resource_type: &str,
        data: &JsonValue,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Serialize once and delegate to payload-based helper
        let content = data.to_string();
        let payload = serde_json::to_string(data).unwrap_or_default();
        let ts = timestamp;

        self.add_resource_payload(id, resource_type, &content, &payload, ts)
            .await
    }

    /// Add resource using already-serialized payload JSON.
    pub async fn add_resource_payload(
        &self,
        id: &str,
        resource_type: &str,
        _content: &str,
        payload_json: &str,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let writer = self.writer.write().await;
        // Ensure we delete any existing document with this ID to prevent duplicates
        writer.delete_term(Term::from_field_text(self.id_field, id));

        let mut doc = doc!(
            self.id_field => id,
            self.type_field => resource_type,
        );

        // Parse JSON and add as JSON object
        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(payload_json) {
            if let Some(obj) = json_val.as_object() {
                let tantivy_obj: BTreeMap<String, OwnedValue> = obj.iter()
                    .map(|(k, v)| (k.clone(), json_to_owned_value(v)))
                    .collect();
                doc.add_object(self.json_field, tantivy_obj);
            }
        }

        if let Some(ts) = timestamp {
            doc.add_date(
                self.timestamp_field,
                tantivy::DateTime::from_timestamp_secs(ts.timestamp()),
            );
        } else {
            let now = Utc::now();
            doc.add_date(
                self.timestamp_field,
                tantivy::DateTime::from_timestamp_secs(now.timestamp()),
            );
        }

        println!("[search] DEBUG: adding doc {:?}", doc);
        writer.add_document(doc)?;
        Ok(())
    }

    /// Delete any indexed document that has the provided id (by term).
    /// This schedules a delete; the periodic committer will flush it.
    pub async fn delete_by_id(&self, id: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        let writer = self.writer.write().await;
        writer.delete_term(Term::from_field_text(self.id_field, id));
        Ok(())
    }

    /// Perform a search and return structured SearchResult rows.
    /// This hydrates the result by fetching event/resource data from the provided Storage.
    pub async fn search(
        &self,
        storage: &Storage,
        query_str: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn Error + Send + Sync>> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::Manual)
            .try_into()?;
        if let Err(e) = reader.reload() {
            eprintln!("[search] warning: failed to reload reader: {}", e);
        }
        let searcher = reader.searcher();
    // Build a query parser that searches across both structured fields and the catch-all:
    // prefer searching the catch_all, but include title/description/comment and the legacy content.
    // Build a query parser that searches the stored JSON payload field.
    // This enables structured/JSON-aware queries over the indexed payload.
    let query_parser = QueryParser::for_index(&self.index, vec![self.json_field]);

    let query = query_parser.parse_query(query_str)?;

    let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results: Vec<SearchResult> = Vec::new();

        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

            let id = retrieved_doc
                .get_first(self.id_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let doc_type = retrieved_doc
                .get_first(self.type_field)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            // textual snippet fallback
            // Attempt to read stored JSON payload from the index (if present)
            let payload_opt = retrieved_doc
                .get_first(self.json_field)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // We no longer populate a generic content snippet when payload is present;
            // structured payload (event/resource) will be used for hydration.
            let content: Option<String> = None;

            // Hydrate structured payloads where possible.
            // If payload is present, parse it first (prefer using index payload to avoid DB lookup).
            let mut resource: Option<JsonValue> = None;
            let mut event: Option<CloudEvent> = None;

            if let Some(payload_str) = payload_opt {
                if let Ok(json_val) = serde_json::from_str::<JsonValue>(&payload_str) {
                    // If payload looks like a CloudEvent (has "specversion" and "id"), try to parse
                    if json_val.get("specversion").is_some() && json_val.get("id").is_some() {
                        if let Ok(ev) = serde_json::from_value::<CloudEvent>(json_val.clone()) {
                            event = Some(ev);
                        }
                    } else {
                        // Otherwise assume it's a resource JSON
                        resource = Some(json_val);
                    }
                }
            }

            match doc_type.as_str() {
                "issue" | "comment" | "task" | "planning" | "document" => {
                    if let Ok(Some(json)) = storage.get_resource(&id).await {
                        resource = Some(json);
                    }
                }
                _ => {
                    if let Ok(Some(ev)) = storage.get_event(&id).await {
                        event = Some(ev);
                    }
                }
            }

            // Final fallback: if neither found, try event
            if resource.is_none() && event.is_none() {
                if let Ok(Some(ev)) = storage.get_event(&id).await {
                    event = Some(ev);
                }
            }

            results.push(SearchResult {
                id,
                doc_type,
                content,
                event,
                resource,
            });
        }

        Ok(results)
    }

    /// Convenience search function that returns empty vec on error.
    pub async fn search_best_effort(
        &self,
        storage: &Storage,
        query: &str,
        limit: usize,
    ) -> Vec<SearchResult> {
        match self.search(storage, query, limit).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[search] best-effort search failed: {}", e);
                Vec::new()
            }
        }
    }

    /// Expose the underlying index directory path for reference (if needed).
    /// Force immediate commit of pending index changes.
    ///
    /// This is provided for tests and for situations where a caller needs
    /// deterministic visibility of recently added documents in the index.
    pub async fn commit(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Acquire writer lock, call commit which returns the number of operations flushed (u64).
        // Map successful u64 result to () and map errors into a boxed error type.
        let mut writer = self.writer.write().await;
        writer
            .commit()
            .map(|_n| ())
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)
    }

    pub fn index_path(&self) -> Option<PathBuf> {
        // Tantivy's Index does not reliably expose its on-disk path in a stable cross-platform API.
        // Return None for now; callers that need the path can track it externally.
        None
    }

    /// Apply authorization filter to a query string.
    /// This injects a clause to restrict results to items where the user is involved.
    pub fn apply_authorization_filter(query: &str, user: &str) -> String {
        // We check two paths:
        // 1. json_payload.involved: For resources (Issues) that have the field directly.
        // 2. json_payload.data.resource_data.involved: For events (CloudEvents) where the involved field is inside the resource_data.
        let user_filter = format!(
            "(json_payload.involved:\"{}\" OR json_payload.data.resource_data.involved:\"{}\")",
            user, user
        );
        if query.trim().is_empty() || query.trim() == "*" {
            user_filter
        } else {
            format!("({}) AND {}", query, user_filter)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use tempfile::TempDir;

    // Verify that when we add a resource and index its payload, a search for a term inside
    // the payload returns at least one result. This covers the case where the index schema
    // must include the `json_payload` field and ensures search hydration works.
    #[tokio::test]
    async fn test_search_indexes_payload_and_hydrates() {
        // Create temporary data directory used for both storage and search index
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::new(temp_dir.path()).await.unwrap();

        // Create resource JSON and persist it
        let resource_data = serde_json::json!({
            "title": "Important Issue",
            "description": "This is a critical bug"
        });

        storage
            .store_resource("issue-1", "issue", &resource_data)
            .await
            .unwrap();

        // Create/open the search index pointing at the same temp dir
        let index_path = temp_dir.path().join("search_index");
        let search_index = SearchIndex::open(&index_path, true, std::time::Duration::from_secs(1))
            .expect("failed to open search index for test");

        // Index the stored resource payload into the search index
        let payload = serde_json::to_string(&resource_data).unwrap_or_default();
        search_index
            .add_resource_payload("issue-1", "issue", "", &payload, None)
            .await
            .expect("failed to add resource payload to index");

        // Force a commit so the reader can see the document immediately in the test
        search_index.commit().await.expect("commit failed");

        // Short pause to let the reader reload (should be immediate but be conservative)
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;

        // Search for a term that appears in the payload
        // Note: For JSON fields, we must specify the path in the query if we want to search a specific field,
        // or rely on default field expansion which might behave differently.
        // Let's try explicit path first to verify indexing.
        let results = search_index.search(&storage, "title:important", 10).await.unwrap();

        assert!(
            !results.is_empty(),
            "Expected search to return results, got none"
        );
        // And one of the results should hydrate the resource
        let has_resource = results.iter().any(|r| r.resource.is_some());
        assert!(
            has_resource,
            "Expected at least one result to be hydrated as a resource"
        );
    }



    #[tokio::test]
    async fn test_query_rewriting() -> Result<(), Box<dyn Error + Send + Sync>> {
        let dir = TempDir::new()?;
        let index = SearchIndex::open(dir.path(), true, std::time::Duration::from_secs(1))?;
        let storage = Storage::new(dir.path()).await.unwrap();

        // Add a document with involved field
        let data = serde_json::json!({
            "title": "Test Issue",
            "involved": "involved_user@example.com"
        });
        index.add_resource_doc("evt1", "issue", &data, None).await?;
        index.commit().await?;

        // Test 1: Search with implicit prefix (should be handled by QueryParser default field)
        let results = index
            .search_best_effort(&storage, "involved:involved_user@example.com", 10)
            .await;
        assert_eq!(results.len(), 1, "Should find with implicit prefix");

        // Test 2: Search with known field (should not be rewritten)
        let results = index
            .search_best_effort(&storage, "id:evt1", 10)
            .await;
        assert_eq!(results.len(), 1, "Should find with id (not rewritten)");

        Ok(())
    }

    #[tokio::test]
    async fn test_authorized_issue_search() -> Result<(), Box<dyn Error + Send + Sync>> {
        let dir = TempDir::new()?;
        let index = SearchIndex::open(dir.path(), true, std::time::Duration::from_secs(1))?;
        let storage = Storage::new(dir.path()).await.unwrap();

        // 1. Create an issue where "alice@example.com" is involved
        let issue_data = serde_json::json!({
            "title": "Alice's Issue",
            "involved": ["alice@example.com"]
        });
        index.add_resource_doc("issue-1", "issue", &issue_data, None).await?;

        // 2. Create a comment (just to populate index)
        let comment_data = serde_json::json!({
            "content": "Some comment",
            "parent_id": "issue-1"
        });
        index.add_resource_doc("comment-1", "comment", &comment_data, None).await?;

        // Commit to make documents searchable
        index.commit().await?;

        // 3. Search as Alice (should find the issue)
        let query_alice = SearchIndex::apply_authorization_filter("*", "alice@example.com");
        let results_alice = index.search_best_effort(&storage, &query_alice, 10).await;

        // Should find issue-1
        let found_issue = results_alice.iter().any(|r| r.id == "issue-1");
        assert!(found_issue, "Alice should see her issue");

        // 4. Search as Bob (should NOT find the issue)
        let query_bob = SearchIndex::apply_authorization_filter("*", "bob@example.com");
        let results_bob = index.search_best_effort(&storage, &query_bob, 10).await;

        let found_issue_bob = results_bob.iter().any(|r| r.id == "issue-1");
        assert!(!found_issue_bob, "Bob should NOT see Alice's issue");

        // 5. Verify query construction
        let custom_query = SearchIndex::apply_authorization_filter("title:Alice", "alice@example.com");
        assert!(custom_query.contains("title:Alice"));
        assert!(custom_query.contains("json_payload.involved:\"alice@example.com\""));

        let results_custom = index.search_best_effort(&storage, &custom_query, 10).await;
        assert!(!results_custom.is_empty(), "Should find issue with specific query and auth");

        Ok(())
    }
}

fn json_to_owned_value(v: &JsonValue) -> OwnedValue {
    match v {
        JsonValue::Null => OwnedValue::Null,
        JsonValue::Bool(b) => OwnedValue::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                OwnedValue::I64(i)
            } else if let Some(u) = n.as_u64() {
                OwnedValue::U64(u)
            } else if let Some(f) = n.as_f64() {
                OwnedValue::F64(f)
            } else {
                OwnedValue::Null
            }
        }
        JsonValue::String(s) => OwnedValue::Str(s.clone()),
        JsonValue::Array(arr) => {
            OwnedValue::Array(arr.iter().map(json_to_owned_value).collect())
        }
        JsonValue::Object(obj) => {
            let map: BTreeMap<String, OwnedValue> = obj.iter()
                .map(|(k, v)| (k.clone(), json_to_owned_value(v)))
                .collect();
            OwnedValue::Object(map)
        }
    }
}
