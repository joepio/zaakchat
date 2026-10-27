/// Storage module for persisting events and resources using redb K/V store.
///
/// This component is storage-only: it persists events and resources to the K/V store
/// (redb) and does NOT depend on or manage the search/indexing subsystem. The search
/// subsystem has been moved to `src/search.rs` to remove Tantivy dependencies from
/// the storage layer and to keep concerns separated.
///
/// Notes:
/// - Events are stored under a sequence-keyed table so iteration returns server-ordered events.
/// - Resource records are stored under their resource id.
use redb::{Database, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::path::Path;
use std::sync::Arc;

use crate::schemas::CloudEvent;

// Define redb tables
// EVENTS_BY_SEQ maps zero-padded sequence keys to serialized event records so iteration is lexicographic by sequence
const EVENTS_BY_SEQ_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("events_by_seq");
const RESOURCES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("resources");
/// Meta table for storing counters and small metadata (e.g. last assigned sequence)
const META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("meta");
/// Transaction id per event sequence key (CloudEvents extension `transactionid`).
/// Kept in its own table so existing EventRecords stay readable.
const EVENT_TXIDS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("event_txids");
/// Processed transactions by idempotency key (`source` + event id), for replays and conflicts.
const IDEMPOTENCY_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("idempotency");

/// Record for storing events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub id: String,
    pub event_type: String,
    pub source: String,
    pub subject: Option<String>,
    pub time: Option<String>,
    pub sequence: Option<String>,
    pub data: String, // JSON serialized
}

/// Outcome of a processed transaction, stored under its idempotency key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdempotencyRecord {
    /// The request body as received, to tell a replay apart from a conflicting reuse of the key.
    pub body: String,
    /// The response body that was returned.
    pub response: String,
}

/// A resource change applied as part of a transaction.
#[derive(Debug, Clone)]
pub enum ResourceWrite {
    Upsert {
        id: String,
        resource_type: String,
        data: JsonValue,
    },
    Delete {
        id: String,
    },
}

/// Error from `apply_transaction`.
#[derive(Debug)]
pub enum TransactionError {
    /// The idempotency key was already used; holds the stored outcome.
    AlreadyProcessed(IdempotencyRecord),
    Storage(Box<dyn std::error::Error + Send + Sync>),
}

impl<E: Into<Box<dyn std::error::Error + Send + Sync>>> From<E> for TransactionError {
    fn from(e: E) -> Self {
        TransactionError::Storage(e.into())
    }
}

/// Record for storing resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRecord {
    pub id: String,
    pub resource_type: String, // issue, comment, task, planning, document
    pub data: String,          // JSON serialized
    pub updated_at: String,
}

/// Storage layer combining redb K/V store.
/// Search/indexing responsibilities live in the separate `search` module (src/search.rs).
pub struct Storage {
    db: Arc<Database>,
    /// Absolute path to the data directory used by this storage instance (e.g. ./data).
    /// Kept so higher-level modules (e.g. the search subsystem) can locate index files.
    pub data_dir: std::path::PathBuf,
}

impl Storage {
    /// Create a new storage instance
    pub async fn new(data_dir: &Path) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Create directories
        let db_path = data_dir.join("data.redb");
        let index_path = data_dir.join("search_index");

        tokio::fs::create_dir_all(data_dir).await?;
        tokio::fs::create_dir_all(&index_path).await?;

        // Initialize redb database
        let db = Database::create(&db_path)?;

        // Initialize tables (include meta & sequence tables)
        let write_txn = db.begin_write()?;
        {
            let _ = write_txn.open_table(EVENTS_BY_SEQ_TABLE)?;
            let _ = write_txn.open_table(RESOURCES_TABLE)?;
            let _ = write_txn.open_table(META_TABLE)?;
            let _ = write_txn.open_table(EVENT_TXIDS_TABLE)?;
            let _ = write_txn.open_table(IDEMPOTENCY_TABLE)?;
        }
        write_txn.commit()?;

        // NOTE:
        // Search/indexing implementation has been moved out of the storage layer into a dedicated
        // search module. The storage component is now responsible only for persistent K/V storage
        // (redb) of events and resources.
        //
        // Initialize storage return value (only DB reference is kept here).
        Ok(Self {
            db: Arc::new(db),
            data_dir: data_dir.to_path_buf(),
        })
    }

    /// Store an event in the K/V store (with diagnostic logging) and assign a monotonically increasing sequence.
    /// Returns the assigned sequence string (zero-padded) on success.
    pub async fn store_event(
        &self,
        event: &CloudEvent,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Diagnostic: log attempt to store event
        println!(
            "[storage] attempt store_event: id={} type={} source={}",
            event.id, event.event_type, event.source
        );

        // Assign the sequence and write the record in one write transaction, so a crash
        // can never leave a counted sequence without its event.
        let write_txn = self.db.begin_write()?;
        let seq = Self::next_seq(&write_txn)?;
        Self::insert_event_record(&write_txn, event, seq)?;
        write_txn.commit()?;

        // Diagnostic: confirm persisted to DB
        let seq_key = format!("{:020}", seq);
        println!(
            "[storage] persisted event to DB: id={} seq={}",
            event.id, seq_key
        );

        // Return the assigned sequence key to the caller
        Ok(seq_key)
    }

    /// Increment and return the sequence counter inside an open write transaction.
    fn next_seq(
        wtx: &redb::WriteTransaction,
    ) -> Result<u128, Box<dyn std::error::Error + Send + Sync>> {
        let mut meta = wtx.open_table(META_TABLE)?;
        let last = match meta.get("last_seq")? {
            Some(g) => std::str::from_utf8(g.value())
                .ok()
                .and_then(|s| s.parse::<u128>().ok()),
            None => None,
        };
        let next = last.map(|v| v + 1).unwrap_or(1);
        meta.insert("last_seq", next.to_string().as_bytes())?;
        Ok(next)
    }

    /// Write an event record (and its transaction id, if any) under `seq`.
    fn insert_event_record(
        wtx: &redb::WriteTransaction,
        event: &CloudEvent,
        seq: u128,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let seq_key = format!("{:020}", seq);
        let record = EventRecord {
            id: event.id.clone(),
            event_type: event.event_type.clone(),
            source: event.source.clone(),
            subject: Some(event.subject.clone()),
            time: event.time.clone(),
            sequence: Some(seq.to_string()),
            data: serde_json::to_string(&event.data)?,
        };
        let serialized = bincode::serialize(&record)?;
        wtx.open_table(EVENTS_BY_SEQ_TABLE)?
            .insert(seq_key.as_str(), serialized.as_slice())?;
        if let Some(txid) = &event.transactionid {
            wtx.open_table(EVENT_TXIDS_TABLE)?
                .insert(seq_key.as_str(), txid.as_str())?;
        }
        Ok(seq_key)
    }

    /// Look up a processed transaction by idempotency key.
    pub async fn get_idempotency(
        &self,
        key: &str,
    ) -> Result<Option<IdempotencyRecord>, Box<dyn std::error::Error + Send + Sync>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(IDEMPOTENCY_TABLE)?;
        match table.get(key)? {
            Some(bytes) => Ok(Some(bincode::deserialize(bytes.value())?)),
            None => Ok(None),
        }
    }

    /// Apply a transaction all-or-nothing: assign sequences to `events` and store them,
    /// apply all resource writes, and record the outcome under `idempotency_key`, in one
    /// redb write transaction. `make_response` builds the stored response from the
    /// events once their sequences are known.
    pub async fn apply_transaction(
        &self,
        mut events: Vec<CloudEvent>,
        writes: &[ResourceWrite],
        idempotency_key: &str,
        body: &str,
        make_response: impl FnOnce(&[CloudEvent]) -> String,
    ) -> Result<(Vec<CloudEvent>, String), TransactionError> {
        let write_txn = self.db.begin_write()?;

        // Re-check inside the write transaction: a concurrent request may have won the race.
        if let Some(bytes) = write_txn
            .open_table(IDEMPOTENCY_TABLE)?
            .get(idempotency_key)?
        {
            let rec: IdempotencyRecord = bincode::deserialize(bytes.value())?;
            return Err(TransactionError::AlreadyProcessed(rec));
        }

        for event in events.iter_mut() {
            let seq = Self::next_seq(&write_txn)?;
            event.sequence = Some(Self::insert_event_record(&write_txn, event, seq)?);
        }

        {
            let mut table = write_txn.open_table(RESOURCES_TABLE)?;
            let now = chrono::Utc::now().to_rfc3339();
            for write in writes {
                match write {
                    ResourceWrite::Upsert {
                        id,
                        resource_type,
                        data,
                    } => {
                        let record = ResourceRecord {
                            id: id.clone(),
                            resource_type: resource_type.clone(),
                            data: serde_json::to_string(data)?,
                            updated_at: now.clone(),
                        };
                        table.insert(id.as_str(), bincode::serialize(&record)?.as_slice())?;
                    }
                    ResourceWrite::Delete { id } => {
                        table.remove(id.as_str())?;
                    }
                }
            }
        }

        let response = make_response(&events);
        let rec = IdempotencyRecord {
            body: body.to_string(),
            response: response.clone(),
        };
        write_txn
            .open_table(IDEMPOTENCY_TABLE)?
            .insert(idempotency_key, bincode::serialize(&rec)?.as_slice())?;

        write_txn.commit()?;
        Ok((events, response))
    }

    /// Transaction id stored for an event sequence key, if any.
    fn txid_for(
        read_txn: &redb::ReadTransaction,
        seq_key: &str,
    ) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
        let table = read_txn.open_table(EVENT_TXIDS_TABLE)?;
        Ok(table.get(seq_key)?.map(|g| g.value().to_string()))
    }

    /// Get an event by ID (scan events_by_seq and return the matching event)
    #[allow(dead_code)]
    pub async fn get_event(
        &self,
        id: &str,
    ) -> Result<Option<CloudEvent>, Box<dyn std::error::Error>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(EVENTS_BY_SEQ_TABLE)?;

        let iter = table.iter()?;
        for item in iter {
            let (key, value) = item?;
            let rec: EventRecord = bincode::deserialize(value.value())?;
            if rec.id == id {
                let data: Option<JsonValue> = serde_json::from_str(&rec.data)?;
                let transactionid =
                    Self::txid_for(&read_txn, key.value()).map_err(|e| e.to_string())?;
                return Ok(Some(CloudEvent {
                    specversion: "1.0".to_string(),
                    id: rec.id,
                    source: rec.source,
                    subject: rec.subject.unwrap_or_else(|| "unknown".to_string()),
                    event_type: rec.event_type,
                    time: rec.time,
                    datacontenttype: Some("application/json".to_string()),
                    dataschema: None,
                    dataref: None,
                    sequence: rec.sequence,
                    sequencetype: None,
                    transactionid,
                    data,
                }));
            }
        }

        Ok(None)
    }

    /// Store a resource in the K/V store (with diagnostic logging)
    pub async fn store_resource(
        &self,
        id: &str,
        resource_type: &str,
        data: &JsonValue,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Diagnostic: log attempt to store resource
        println!(
            "[storage] attempt store_resource: id={} type={}",
            id, resource_type
        );

        let timestamp = chrono::Utc::now().to_rfc3339();

        let record = ResourceRecord {
            id: id.to_string(),
            resource_type: resource_type.to_string(),
            data: serde_json::to_string(data)?,
            updated_at: timestamp.clone(),
        };

        let serialized = bincode::serialize(&record)?;

        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(RESOURCES_TABLE)?;
            table.insert(id, serialized.as_slice())?;
        }
        write_txn.commit()?;

        // Diagnostic: confirm persisted to DB
        println!("[storage] persisted resource to DB: id={}", id);

        Ok(())
    }

    /// Get a resource by ID
    pub async fn get_resource(
        &self,
        id: &str,
    ) -> Result<Option<JsonValue>, Box<dyn std::error::Error + Send + Sync>> {
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(RESOURCES_TABLE)?;

        let result = table.get(id)?;

        match result {
            Some(bytes) => {
                let rec: ResourceRecord = bincode::deserialize(bytes.value())?;
                let data: JsonValue = serde_json::from_str(&rec.data)?;
                Ok(Some(data))
            }
            None => Ok(None),
        }
    }

    /// Delete a resource
    pub async fn delete_resource(
        &self,
        id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = write_txn.open_table(RESOURCES_TABLE)?;
            table.remove(id)?;
        }
        write_txn.commit()?;

        // Note: search/index removal is handled by the search subsystem (src/search.rs).
        // Storage no longer directly manipulates the index.

        Ok(())
    }

    /// Clear all data from storage (events, resources, and metadata)
    pub async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let write_txn = self.db.begin_write()?;
        {
            // Clear events table
            let mut events_table = write_txn.open_table(EVENTS_BY_SEQ_TABLE)?;
            // Collect keys first to avoid iterator invalidation issues
            let keys: Vec<String> = events_table
                .iter()?
                .map(|r| r.map(|(k, _)| k.value().to_string()))
                .collect::<Result<_, _>>()?;
            for key in keys {
                events_table.remove(key.as_str())?;
            }

            // Clear resources table
            let mut resources_table = write_txn.open_table(RESOURCES_TABLE)?;
            let keys: Vec<String> = resources_table
                .iter()?
                .map(|r| r.map(|(k, _)| k.value().to_string()))
                .collect::<Result<_, _>>()?;
            for key in keys {
                resources_table.remove(key.as_str())?;
            }

            // Reset meta table (sequence counter)
            let mut meta_table = write_txn.open_table(META_TABLE)?;
            meta_table.remove("last_seq")?;
        }
        write_txn.commit()?;

        println!("[storage] cleared all data");
        Ok(())
    }

    // Note: indexing is performed asynchronously by background tasks and commits are batched periodically.

    /// Search/index responsibilities have been moved to `src/search.rs`.
    /// Storage is now purely a K/V persistence layer and does not expose any
    /// direct references to the search/index internals (no Tantivy types or fields).
    ///
    /// The search subsystem (index writer, schema fields, periodic committer,
    /// and add/search/delete operations) lives in the dedicated search module.
    ///
    /// Convenience compatibility wrapper: allow older test code (and other callers)
    /// to call `storage.search(...)`. This delegates to the search subsystem by
    /// opening the search index located under `data_dir/search_index` and performing
    /// the search there. This wrapper exists for backward compatibility; callers
    /// are encouraged to use the `SearchIndex` API directly (via AppState.search).
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, Box<dyn std::error::Error + Send + Sync>> {
        // Construct index path relative to storage's data_dir
        let index_path = self.data_dir.join("search_index");
        // Open a temporary SearchIndex instance and delegate search.
        // Note: open() is designed to be cheap for read-only operations and will reuse
        // the existing on-disk index. If you prefer, higher-level code can keep a
        // long-lived SearchIndex instance (recommended for production).
        let search_index = crate::search::SearchIndex::open(
            &index_path,
            true,
            std::time::Duration::from_secs(10),
        )?;
        let results = search_index.search(self, query, limit).await?;
        Ok(results)
    }

    /// Get all resources (paginated)
    pub async fn list_resources(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<(String, JsonValue)>, Box<dyn std::error::Error + Send + Sync>> {
        let mut results = Vec::new();

        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(RESOURCES_TABLE)?;

        let mut count = 0;
        let iter = table.iter()?;

        for item in iter {
            let (key, value) = item?;

            if count >= offset {
                let rec: ResourceRecord = bincode::deserialize(value.value())?;
                let data: JsonValue = serde_json::from_str(&rec.data)?;
                results.push((key.value().to_string(), data));

                if results.len() >= limit {
                    break;
                }
            }
            count += 1;
        }

        Ok(results)
    }

    /// List events by sequence with pagination after a given sequence key.
    ///
    /// This function returns events in backend processing order (ascending by sequence).
    /// Use `after_seq` to fetch events after a particular zero-padded sequence key
    /// (e.g. "00000000000000000042"). If `after_seq` is `None`, iteration starts at the beginning.
    pub async fn list_events_after(
        &self,
        after_seq: Option<String>,
        limit: usize,
    ) -> Result<Vec<CloudEvent>, Box<dyn std::error::Error + Send + Sync>> {
        // Read events by sequence lexicographic order from EVENTS_BY_SEQ_TABLE (ensures server processing order).
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(EVENTS_BY_SEQ_TABLE)?;

        let mut results: Vec<CloudEvent> = Vec::new();

        let iter = table.iter()?;

        for item in iter {
            let (key, value) = item?;
            // If after_seq is provided, skip until key > after_seq
            if let Some(ref after) = after_seq {
                // key.value() returns &str; compare lexicographically
                if key.value() <= after.as_str() {
                    continue;
                }
            }

            let rec: EventRecord = bincode::deserialize(value.value())?;
            let data: Option<JsonValue> = serde_json::from_str(&rec.data)?;
            let transactionid = Self::txid_for(&read_txn, key.value())?;

            let event = CloudEvent {
                specversion: "1.0".to_string(),
                id: rec.id,
                source: rec.source,
                subject: rec.subject.unwrap_or_else(|| "unknown".to_string()),
                event_type: rec.event_type,
                time: rec.time,
                datacontenttype: Some("application/json".to_string()),
                dataschema: None,
                dataref: None,
                sequence: rec.sequence,
                sequencetype: None,
                transactionid,
                data,
            };

            results.push(event);
            if results.len() >= limit {
                break;
            }
        }

        Ok(results)
    }

    /// Backwards-compatible wrapper: list events by offset (legacy).
    /// This calls `list_events_after` by computing `after_seq` from offset = number to skip.
    /// Note: this wrapper is less efficient for large offsets and is provided for compatibility.
    pub async fn list_events(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<CloudEvent>, Box<dyn std::error::Error + Send + Sync>> {
        // If offset is zero, simply return first `limit` events
        if offset == 0 {
            return self.list_events_after(None, limit).await;
        }

        // Otherwise, we need to skip `offset` keys - iterate and find the key at position `offset - 1`
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(EVENTS_BY_SEQ_TABLE)?;
        let iter = table.iter()?;

        let mut seq_to_start: Option<String> = None;
        for (i, item) in iter.enumerate() {
            let (key, _value) = item?;
            if i + 1 == offset {
                seq_to_start = Some(key.value().to_string());
                break;
            }
        }

        // If we found the sequence key at offset-1, start after it; otherwise start from beginning
        let after_seq = seq_to_start.map(|s| s);
        self.list_events_after(after_seq, limit).await
    }
}

/// Search result structure
///
/// `content` is optional and will be omitted when a structured `event` or `resource`
/// is present. Clients should prefer `event` or `resource` when available.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The identifier for the match. For resources this is the resource id; for events it's the event id.
    pub id: String,
    /// The document type stored in the index (e.g. "issue", "comment", "nl.vng.zaken.json-commit.v1", etc.)
    pub doc_type: String,
    /// A simple textual snippet representing the indexed content (kept for backward compatibility).
    /// This field is optional and will be omitted from serialized output when a structured
    /// `event` or `resource` is present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// If the match corresponds to a persisted CloudEvent, this will be populated.
    /// Clients can prefer this structured CloudEvent over the textual `content`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<CloudEvent>,
    /// If the match corresponds to a persisted resource (issue/comment/task/etc),
    /// this will contain the parsed JSON resource.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<JsonValue>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_storage_event_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::new(temp_dir.path()).await.unwrap();

        let event = CloudEvent {
            specversion: "1.0".to_string(),
            id: "test-event-1".to_string(),
            source: "test".to_string(),
            subject: "test-subject".to_string(),
            event_type: "test.event".to_string(),
            time: Some(chrono::Utc::now().to_rfc3339()),
            datacontenttype: Some("application/json".to_string()),
            dataschema: None,
            dataref: None,
            sequence: Some("1".to_string()),
            sequencetype: None,
            transactionid: None,
            data: Some(serde_json::json!({"key": "value"})),
        };

        let _seq = storage.store_event(&event).await.unwrap();

        let retrieved = storage.get_event("test-event-1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "test-event-1");
    }

    #[tokio::test]
    async fn test_storage_resource_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::new(temp_dir.path()).await.unwrap();

        let resource_data = serde_json::json!({
            "title": "Test Issue",
            "status": "open"
        });

        storage
            .store_resource("issue-1", "issue", &resource_data)
            .await
            .unwrap();

        let retrieved = storage.get_resource("issue-1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap()["title"], "Test Issue");
    }

    #[tokio::test]
    async fn test_search() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::new(temp_dir.path()).await.unwrap();

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
        let search_index =
            crate::search::SearchIndex::open(&index_path, true, std::time::Duration::from_secs(1))
                .expect("failed to open search index for test");

        // Index the stored resource payload into the search index
        let payload = serde_json::to_string(&resource_data).unwrap_or_default();
        search_index
            .add_resource_payload("issue-1", "issue", "", &payload, None)
            .await
            .expect("failed to add resource payload to index");

        // Force a commit so the reader can see the document immediately in the test
        search_index.commit().await.expect("commit failed");

        // Call the SearchIndex directly and assert structured results are returned.
        let results = search_index
            .search(&storage, "title:important", 10)
            .await
            .unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_list_resources() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::new(temp_dir.path()).await.unwrap();

        // Add multiple resources
        for i in 1..=5 {
            let resource_data = serde_json::json!({
                "title": format!("Issue {}", i),
                "status": "open"
            });
            storage
                .store_resource(&format!("issue-{}", i), "issue", &resource_data)
                .await
                .unwrap();
        }

        // List all
        let all_resources = storage.list_resources(0, 10).await.unwrap();
        assert_eq!(all_resources.len(), 5);

        // List with pagination
        let page1 = storage.list_resources(0, 2).await.unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = storage.list_resources(2, 2).await.unwrap();
        assert_eq!(page2.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_resource() {
        let temp_dir = TempDir::new().unwrap();
        let storage = Storage::new(temp_dir.path()).await.unwrap();

        let resource_data = serde_json::json!({
            "title": "Test Issue",
            "status": "open"
        });

        storage
            .store_resource("issue-1", "issue", &resource_data)
            .await
            .unwrap();

        // Verify it exists
        let retrieved = storage.get_resource("issue-1").await.unwrap();
        assert!(retrieved.is_some());

        // Delete it
        storage.delete_resource("issue-1").await.unwrap();

        // Verify it's gone
        let retrieved = storage.get_resource("issue-1").await.unwrap();
        assert!(retrieved.is_none());
    }
}
