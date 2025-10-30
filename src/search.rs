use std::{
    error::Error,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use tantivy::collector::TopDocs;
use tantivy::directory::MmapDirectory;
use tantivy::query::QueryParser;
use tantivy::schema::*;
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
    content_field: Field,
    // Payload field holds stored JSON for direct hydration
    payload_field: Field,
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
        let _dir = MmapDirectory::open(index_path)?;
        // Build schema
        let mut schema_builder = Schema::builder();
        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let type_field = schema_builder.add_text_field("type", STRING | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        // Store payload JSON as a stored text field (parsed by search module when available)
        let payload_field = schema_builder.add_text_field("payload", STORED);
        let timestamp_field = schema_builder.add_date_field("timestamp", INDEXED | STORED);
        let schema = schema_builder.build();

        // Create or open index
        let index = if index_path.exists() && index_path.read_dir()?.next().is_some() {
            Index::open_in_dir(index_path)?
        } else {
            Index::create_in_dir(index_path, schema.clone())?
        };

        let writer = index.writer(50_000_000)?; // 50 MB heap for writer

        let si = Self {
            index: Arc::new(index),
            writer: Arc::new(RwLock::new(writer)),
            id_field,
            type_field,
            content_field,
            payload_field,
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
                    } else {
                        println!("[search][committer] commit completed");
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
    /// This function will add the document to the writer but will not commit.
    pub async fn add_event_doc(
        &self,
        event: &CloudEvent,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Build textual content
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

        // Also prepare a stored JSON payload (full CloudEvent) for structured hydration
        let payload = match serde_json::to_string(&event) {
            Ok(s) => s,
            Err(_) => String::new(),
        };

        let writer = self.writer.write().await;

        let mut doc = doc!(
            self.id_field => event.id.as_str(),
            self.type_field => event.event_type.as_str(),
            self.content_field => content.as_str(),
            self.payload_field => payload.as_str(),
        );

        if let Some(ts_str) = &event.time {
            if let Ok(dt) = DateTime::parse_from_rfc3339(ts_str) {
                let utc = dt.with_timezone(&Utc);
                doc.add_date(
                    self.timestamp_field,
                    tantivy::DateTime::from_timestamp_secs(utc.timestamp()),
                );
            }
        } else {
            // no time, use now
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
    /// `data` is the full JSON object for the resource.
    pub async fn add_resource_doc(
        &self,
        id: &str,
        resource_type: &str,
        data: &JsonValue,
        timestamp: Option<DateTime<Utc>>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let content = data.to_string();
        // Store payload JSON as well (so search can return structured resource without extra DB lookup)
        let payload = match serde_json::to_string(data) {
            Ok(s) => s,
            Err(_) => String::new(),
        };

        let writer = self.writer.write().await;

        let mut doc = doc!(
            self.id_field => id,
            self.type_field => resource_type,
            self.content_field => content.as_str(),
            self.payload_field => payload.as_str(),
        );

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
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(&self.index, vec![self.content_field]);
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
            let content = retrieved_doc
                .get_first(self.content_field)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Attempt to read stored JSON payload from index (if present)
            let payload_opt = retrieved_doc
                .get_first(self.payload_field)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

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
    pub fn index_path(&self) -> Option<PathBuf> {
        // Tantivy's Index does not directly expose its on-disk path in a guaranteed way,
        // but we keep this for future use if necessary.
        None
    }
}
