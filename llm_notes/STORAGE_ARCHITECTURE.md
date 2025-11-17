# Storage Architecture

This document describes the persistent storage architecture for the SSE Demo application, which provides three main endpoints for Command/Sync operations, resource retrieval, and complex queries.

## Overview

The application uses a combination of:
- **redb**: Embedded key-value database for persisting events and resources
- **Tantivy**: Full-text search engine for complex queries and filtering
- **Server-Sent Events (SSE)**: Real-time updates broadcast to connected clients

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                      HTTP Endpoints                          │
├─────────────────────────────────────────────────────────────┤
│  POST /events        │  GET /resources     │  GET /query    │
│  (Command + Sync)    │  GET /resources/:id │  (Search)      │
│                      │  DELETE /resources  │                │
└──────────┬───────────┴─────────┬───────────┴────────┬───────┘
           │                     │                     │
           ▼                     ▼                     ▼
┌─────────────────────────────────────────────────────────────┐
│                     Storage Layer                            │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌──────────────────┐    ┌──────────────────┐              │
│  │   redb K/V Store │    │  Tantivy Index   │              │
│  ├──────────────────┤    ├──────────────────┤              │
│  │ Events Table     │    │ Full-text search │              │
│  │ Resources Table  │    │ Filtering        │              │
│  └──────────────────┘    └──────────────────┘              │
│                                                               │
└─────────────────────────────────────────────────────────────┘
           │                     │
           ▼                     ▼
    ┌──────────────────────────────────┐
    │  Broadcast Channel (SSE Stream)  │
    └──────────────────────────────────┘
```

## Endpoints

### 1. POST /events - Command + Sync

This is the primary endpoint for creating, updating, and deleting resources through CloudEvents.

**Purpose**: 
- Receive CloudEvents following the CloudEvents 1.0 specification
- Process events to create/update/delete resources
- Broadcast changes to SSE subscribers
- Store events in the persistent log

**Request Format**:
```json
{
  "specversion": "1.0",
  "id": "event-123",
  "source": "nl.vng.zaken",
  "type": "nl.vng.zaken.json-commit.v1",
  "subject": "issue/abc-123",
  "time": "2024-01-15T10:30:00Z",
  "datacontenttype": "application/json",
  "data": {
    "schema": "http://localhost:8000/schemas/Issue",
    "resource_id": "abc-123",
    "actor": "user@example.com",
    "timestamp": "2024-01-15T10:30:00Z",
    "resource_data": {
      "title": "New Issue",
      "status": "open"
    },
    "patch": null,
    "deleted": false
  }
}
```

**Event Processing Flow**:
1. Event is stored in the events table
2. Event data is parsed to extract resource changes
3. For `json-commit` events:
   - If `deleted: true`, resource is removed
   - If `patch` is present, JSON Merge Patch (RFC 7396) is applied
   - If `resource_data` is present, resource is created/replaced
4. Resource is stored in the resources table
5. Search index is updated
6. Event is broadcast to SSE subscribers

### 2. GET /resources - Resource Retrieval

Individual resource access with pagination support.

**Endpoints**:

#### List Resources (Paginated)
```http
GET /resources?offset=0&limit=50
```

**Response**:
```json
[
  {
    "id": "issue-123",
    "resource_type": "issue",
    "data": {
      "title": "Example Issue",
      "status": "open"
    }
  }
]
```

#### Get Single Resource
```http
GET /resources/issue-123
```

**Response**:
```json
{
  "title": "Example Issue",
  "status": "open",
  "assignee": "user@example.com"
}
```

#### Delete Resource
```http
DELETE /resources/issue-123
```

**Response**: `204 No Content`

### 3. GET /query - Complex Queries

Full-text search powered by Tantivy for complex queries and filtering.

**Endpoint**:
```http
GET /query?q=critical+bug&limit=20
```

**Query Parameters**:
- `q`: Search query string (full-text search)
- `limit`: Maximum number of results (default: 50)

**Response**:
```json
{
  "query": "critical bug",
  "count": 3,
  "results": [
    {
      "id": "issue-456",
      "doc_type": "issue",
      "content": "{\"title\":\"Critical Bug\",\"description\":\"System crash\"}"
    }
  ]
}
```

**Search Capabilities**:
- Full-text search across all resource fields
- Searches both events and resources
- Supports Tantivy query syntax for advanced queries

## Data Storage

### redb Tables

#### Events Table
**Key**: Event ID (string)
**Value**: Serialized `EventRecord`

```rust
struct EventRecord {
    id: String,
    event_type: String,
    source: String,
    subject: Option<String>,
    time: Option<String>,
    sequence: Option<String>,
    data: String, // JSON serialized
}
```

#### Resources Table
**Key**: Resource ID (string)
**Value**: Serialized `ResourceRecord`

```rust
struct ResourceRecord {
    id: String,
    resource_type: String, // issue, comment, task, planning, document
    data: String,          // JSON serialized
    updated_at: String,
}
```

### Tantivy Search Index

**Schema**:
- `id` (TEXT, STORED): Unique identifier
- `type` (TEXT, STORED): Resource or event type
- `content` (TEXT): Searchable content (full resource/event data)
- `timestamp` (DATE, INDEXED): Creation/update timestamp

## Event Processing Logic

### JSON Commit Events

The primary event type is `nl.vng.zaken.json-commit.v1`, which follows this structure:

```rust
struct JSONCommit {
    schema: Option<String>,
    resource_id: String,
    actor: Option<String>,
    timestamp: Option<String>,
    resource_data: Option<Value>,
    patch: Option<Value>,
    deleted: Option<bool>,
}
```

**Processing Rules**:

1. **Creation**: When `resource_data` is provided and resource doesn't exist
   - Store full resource data
   - Index for search
   - Broadcast event

2. **Update (Full Replace)**: When `resource_data` is provided and resource exists
   - Replace entire resource
   - Update search index
   - Broadcast event

3. **Update (Patch)**: When `patch` is provided
   - Apply JSON Merge Patch (RFC 7396) to existing resource
   - Update search index
   - Broadcast event

4. **Deletion**: When `deleted: true`
   - Remove from resources table
   - Remove from search index
   - Broadcast event

### JSON Merge Patch (RFC 7396)

The patch mechanism follows RFC 7396:

**Example**:
```json
// Original Resource
{
  "title": "Old Title",
  "status": "open",
  "nested": { "a": 1, "b": 2 }
}

// Patch
{
  "title": "New Title",
  "status": null,
  "nested": { "b": 3, "c": 4 }
}

// Result
{
  "title": "New Title",
  "nested": { "a": 1, "b": 3, "c": 4 }
}
```

- Setting a field to `null` removes it
- Nested objects are recursively merged
- Arrays are replaced, not merged

## Real-time Updates (SSE)

The application maintains a broadcast channel that pushes events to all connected SSE clients.

**SSE Endpoint**: `GET /events/stream`

**Message Types**:

1. **Snapshot** (on connection):
```
event: snapshot
data: [{"id":"event-1",...}]
```

2. **Delta** (on change):
```
event: delta
data: {"id":"event-2","type":"json.commit",...}
```

## Running the Application

### Storage-Enabled Version

```bash
# Build and run with storage
cargo run --bin sse-delta-snapshot-storage

# With environment variables
DATA_DIR=./my_data cargo run --bin sse-delta-snapshot-storage

# With demo mode (generates random events)
DEMO=1 cargo run --bin sse-delta-snapshot-storage
```

### Environment Variables

- `DATA_DIR`: Directory for storing database and index files (default: `./data`)
- `BASE_URL`: Base URL for schema references (default: `http://localhost:8000`)
- `DEMO`: Enable demo mode with automatic event generation

### Directory Structure

```
data/
├── data.redb           # redb database file
└── search_index/       # Tantivy index directory
    ├── .managed.json
    ├── meta.json
    └── *.idx
```

## Resource Types

The system recognizes the following resource types:

1. **Issue**: Issue/ticket tracking
2. **Comment**: Comments on issues
3. **Task**: Action items
4. **Planning**: Planning items with moments
5. **Document**: Document references

Resource type is determined from:
1. The `schema` field in JSONCommit events
2. The `subject` field in CloudEvents (fallback)

## Example Workflows

### Creating an Issue

```bash
curl -X POST http://localhost:8000/events \
  -H "Content-Type: application/json" \
  -d '{
    "specversion": "1.0",
    "id": "event-001",
    "source": "client-app",
    "type": "nl.vng.zaken.json-commit.v1",
    "subject": "issue/issue-001",
    "data": {
      "resource_id": "issue-001",
      "resource_data": {
        "title": "New Feature Request",
        "status": "open",
        "description": "Add dark mode"
      }
    }
  }'
```

### Updating an Issue (Patch)

```bash
curl -X POST http://localhost:8000/events \
  -H "Content-Type: application/json" \
  -d '{
    "specversion": "1.0",
    "id": "event-002",
    "source": "client-app",
    "type": "nl.vng.zaken.json-commit.v1",
    "subject": "issue/issue-001",
    "data": {
      "resource_id": "issue-001",
      "patch": {
        "status": "in-progress"
      }
    }
  }'
```

### Retrieving an Issue

```bash
curl http://localhost:8000/resources/issue-001
```

### Searching Issues

```bash
curl "http://localhost:8000/query?q=dark+mode&limit=10"
```

### Deleting an Issue

```bash
# Via DELETE endpoint
curl -X DELETE http://localhost:8000/resources/issue-001

# Or via event
curl -X POST http://localhost:8000/events \
  -H "Content-Type: application/json" \
  -d '{
    "specversion": "1.0",
    "id": "event-003",
    "source": "client-app",
    "type": "nl.vng.zaken.json-commit.v1",
    "subject": "issue/issue-001",
    "data": {
      "resource_id": "issue-001",
      "deleted": true
    }
  }'
```

## Testing

Run the storage tests:

```bash
cargo test --lib storage
```

Run the handler tests:

```bash
cargo test --lib handlers
```

## Performance Considerations

### redb
- Embedded database, no separate server needed
- ACID transactions
- Memory-mapped files for efficient I/O
- Suitable for moderate write loads

### Tantivy
- Inverted index for fast full-text search
- 50MB heap for index writer (configurable)
- Automatic commit after each write (can be batched for higher throughput)

### Recommendations
- For high-throughput scenarios, consider batching writes
- Monitor disk usage as both event log and resources grow
- Consider compaction/cleanup strategies for old events
- Use pagination for large result sets

## Future Enhancements

1. **Advanced Queries**: Support for filtering by resource type, date ranges, etc.
2. **Event Replay**: Ability to rebuild resource state from event log
3. **Snapshots**: Periodic snapshots to reduce event log size
4. **Replication**: Multi-node setup for high availability
5. **Compression**: Compress old events to save space
6. **Metrics**: Add instrumentation for monitoring performance
7. **Webhooks**: Allow external systems to subscribe to events
8. **Access Control**: Add authentication and authorization

## Migration from In-Memory Version

The original version (`src/main.rs`) keeps everything in memory. To migrate:

1. Run the storage-enabled version: `cargo run --bin sse-delta-snapshot-storage`
2. The storage version maintains the same API surface
3. Events are automatically persisted to disk
4. On restart, data is loaded from the database

Both versions can coexist during transition period.