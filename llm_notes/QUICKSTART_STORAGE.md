# Quick Start Guide - Storage-Backed SSE Demo

This guide will help you get started with the storage-backed version of the SSE Demo application.

## Prerequisites

- Rust 1.70+ installed
- `cargo` package manager

## Installation

1. **Clone the repository** (if not already done):
```bash
git clone <repository-url>
cd sse-demo
```

2. **Install dependencies**:
```bash
cargo build --bin sse-delta-snapshot-storage
```

## Running the Application

### Basic Run

```bash
cargo run --bin sse-delta-snapshot-storage
```

The server will start on `http://localhost:8000`

### With Demo Mode

Demo mode generates random events every 10 seconds and resets the database every 5 minutes:

```bash
DEMO=1 cargo run --bin sse-delta-snapshot-storage
```

### Custom Data Directory

```bash
DATA_DIR=/path/to/data cargo run --bin sse-delta-snapshot-storage
```

### Production Mode

```bash
cargo build --release --bin sse-delta-snapshot-storage
DATA_DIR=/var/lib/sse-demo ./target/release/sse-delta-snapshot-storage
```

## Verify Installation

Once running, test the endpoints:

### 1. Check Resources
```bash
curl http://localhost:8000/resources
```

### 2. Create an Issue
```bash
curl -X POST http://localhost:8000/events \
  -H "Content-Type: application/json" \
  -d '{
    "specversion": "1.0",
    "id": "quickstart-001",
    "source": "quickstart",
    "type": "nl.vng.zaken.json-commit.v1",
    "subject": "issue/my-first-issue",
    "data": {
      "resource_id": "my-first-issue",
      "resource_data": {
        "title": "My First Issue",
        "status": "open",
        "description": "Testing the storage system"
      }
    }
  }'
```

### 3. Retrieve the Issue
```bash
curl http://localhost:8000/resources/my-first-issue
```

### 4. Search for It
```bash
curl "http://localhost:8000/query?q=First+Issue"
```

### 5. Update the Issue (Patch)
```bash
curl -X POST http://localhost:8000/events \
  -H "Content-Type: application/json" \
  -d '{
    "specversion": "1.0",
    "id": "quickstart-002",
    "source": "quickstart",
    "type": "nl.vng.zaken.json-commit.v1",
    "subject": "issue/my-first-issue",
    "data": {
      "resource_id": "my-first-issue",
      "patch": {
        "status": "in-progress"
      }
    }
  }'
```

### 6. Verify the Update
```bash
curl http://localhost:8000/resources/my-first-issue | jq
```

Should show:
```json
{
  "title": "My First Issue",
  "status": "in-progress",
  "description": "Testing the storage system"
}
```

### 7. Delete the Issue
```bash
curl -X DELETE http://localhost:8000/resources/my-first-issue
```

## Connecting via SSE

### Using curl
```bash
curl -N http://localhost:8000/events/stream
```

You'll receive:
1. A `snapshot` event with all current events
2. `delta` events as changes occur

### Using JavaScript

```javascript
const eventSource = new EventSource('http://localhost:8000/events/stream');

eventSource.addEventListener('snapshot', (event) => {
  const snapshot = JSON.parse(event.data);
  console.log('Initial snapshot:', snapshot);
});

eventSource.addEventListener('delta', (event) => {
  const delta = JSON.parse(event.data);
  console.log('New event:', delta);
});
```

## Understanding the Endpoints

### POST /events
**Purpose**: Command + Sync - create, update, delete resources

**Input**: CloudEvent JSON
**Output**: Accepted (202) + broadcasts to SSE clients

### GET /resources
**Purpose**: List all resources (paginated)

**Parameters**: 
- `offset` (default: 0)
- `limit` (default: 50)

**Output**: Array of resources with IDs

### GET /resources/:id
**Purpose**: Get a specific resource

**Output**: Resource data as JSON

### DELETE /resources/:id
**Purpose**: Delete a resource

**Output**: 204 No Content

### GET /query
**Purpose**: Full-text search across all resources and events

**Parameters**:
- `q`: Search query (required)
- `limit`: Max results (default: 50)

**Output**: Search results with relevance ranking

### GET /events/stream
**Purpose**: Server-Sent Events stream for real-time updates

**Output**: SSE stream with snapshot + deltas

## Data Persistence

All data is stored in the `DATA_DIR` directory (default: `./data`):

```
data/
├── data.redb           # Embedded database (events + resources)
└── search_index/       # Full-text search index
```

**Important**: This directory persists between restarts. To reset:

```bash
rm -rf data/
cargo run --bin sse-delta-snapshot-storage
```

## Common Operations

### Creating Different Resource Types

**Comment**:
```bash
curl -X POST http://localhost:8000/events \
  -H "Content-Type: application/json" \
  -d '{
    "specversion": "1.0",
    "id": "comment-001",
    "source": "quickstart",
    "type": "nl.vng.zaken.json-commit.v1",
    "subject": "comment/my-comment",
    "data": {
      "resource_id": "my-comment",
      "resource_data": {
        "content": "This is a comment",
        "parent_id": "my-first-issue"
      }
    }
  }'
```

**Task**:
```bash
curl -X POST http://localhost:8000/events \
  -H "Content-Type: application/json" \
  -d '{
    "specversion": "1.0",
    "id": "task-001",
    "source": "quickstart",
    "type": "nl.vng.zaken.json-commit.v1",
    "subject": "task/my-task",
    "data": {
      "resource_id": "my-task",
      "resource_data": {
        "cta": "Review PR",
        "description": "Review the pull request for feature X",
        "completed": false
      }
    }
  }'
```

### Pagination

List resources in pages:
```bash
# First page (items 0-9)
curl "http://localhost:8000/resources?offset=0&limit=10"

# Second page (items 10-19)
curl "http://localhost:8000/resources?offset=10&limit=10"
```

### Advanced Search

Tantivy supports powerful query syntax:

```bash
# Phrase search
curl "http://localhost:8000/query?q=%22critical+bug%22"

# AND operator
curl "http://localhost:8000/query?q=open+AND+urgent"

# OR operator
curl "http://localhost:8000/query?q=bug+OR+issue"
```

## Troubleshooting

### Port Already in Use
```bash
# Use a different port
PORT=8080 cargo run --bin sse-delta-snapshot-storage
```

### Database Locked
If you see database lock errors, ensure only one instance is running.

### Search Index Corruption
Delete and rebuild:
```bash
rm -rf data/search_index/
# Restart the application - it will rebuild the index
```

### Frontend Missing
If you see "Frontend dist folder is missing":
```bash
cd frontend
pnpm install
pnpm run build
cd ..
```

## Next Steps

- Read [STORAGE_ARCHITECTURE.md](./STORAGE_ARCHITECTURE.md) for detailed architecture
- Explore the API using the AsyncAPI docs at `http://localhost:8000/asyncapi-docs`
- Review the test files for more examples:
  - `src/storage.rs` - Storage layer tests
  - `src/handlers.rs` - Handler tests

## Comparison with In-Memory Version

| Feature | In-Memory (`main.rs`) | Storage (`main_with_storage.rs`) |
|---------|----------------------|-----------------------------------|
| Persistence | ❌ Lost on restart | ✅ Survives restarts |
| Search | ❌ No search | ✅ Full-text search |
| Scalability | Limited by RAM | Limited by disk |
| Query API | ❌ No | ✅ Yes |
| Resource API | ❌ No | ✅ Yes |
| SSE Stream | ✅ Yes | ✅ Yes |
| Event Log | In-memory array | Persistent database |

## Performance Tips

1. **Batch Operations**: If inserting many resources, consider batching events
2. **Query Optimization**: Use specific search terms instead of broad queries
3. **Index Tuning**: Adjust the search writer heap size in `storage.rs` if needed
4. **Monitor Disk**: Set up alerts for disk usage in production

## Getting Help

- Check the logs for error messages
- Run tests: `cargo test`
- Review the source code in `src/handlers.rs` and `src/storage.rs`
- See examples in the test functions

## Demo Mode Details

When `DEMO=1` is set:
- Generates a random event every 10 seconds
- Events include: issue updates, comments, tasks, planning items
- Resets entire database every 5 minutes
- Useful for testing SSE clients and seeing the system in action

```bash
# Watch demo events in real-time
DEMO=1 cargo run --bin sse-delta-snapshot-storage &
curl -N http://localhost:8000/events/stream
```

## Production Checklist

Before deploying to production:

- [ ] Set `DATA_DIR` to a persistent location
- [ ] Configure `BASE_URL` to your domain
- [ ] Remove `DEMO=1` environment variable
- [ ] Set up log rotation
- [ ] Monitor disk usage
- [ ] Set up backup strategy for `data/` directory
- [ ] Configure firewall/reverse proxy
- [ ] Enable HTTPS
- [ ] Consider authentication/authorization
- [ ] Set up monitoring and alerting
- [ ] Test disaster recovery procedures

Happy coding! 🚀