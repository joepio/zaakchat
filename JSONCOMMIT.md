# JSONCommit Documentation

`JSONCommit` is a unified event type designed to simplify the representation of changes to JSON resources. Instead of having separate event types for "create", "update", and "delete" operations, `JSONCommit` uses a single event type where the presence or absence of specific fields determines the nature of the change.

## The Core Concept

A `json.commit` event represents a single "commit" of changes to a resource, much like a Git commit.

- **Create**: A new resource is created. The `resource_data` field contains the initial state of the resource.
- **Update**: An existing resource is modified. The `patch` field contains a JSON Merge Patch ([RFC 7396](https://tools.ietf.org/html/rfc7396)) describing the changes.
- **Delete**: A resource is removed. The `deleted` field is set to `true`.

## Event Structure

All events in the system follow the [CloudEvents](https://cloudevents.io/) specification. A `JSONCommit` is wrapped within the `data` field of a CloudEvent.

### CloudEvent Envelope

| Field | Type | Description |
| :--- | :--- | :--- |
| `specversion` | `string` | Always "1.0". |
| `id` | `string` | A unique identifier for the event. |
| `type` | `string` | For JSONCommits, this is typically `json.commit` or `nl.vng.zaken.json-commit.v1`. |
| `source` | `string` | The system or component that generated the event. |
| `subject` | `string` | The ID of the primary resource (e.g., the `zaakId`). |
| `time` | `string` | ISO 8601 timestamp of event occurrence. |
| `data` | `object` | The `JSONCommit` payload. |

### JSONCommit Payload

| Field | Type | Description |
| :--- | :--- | :--- |
| `schema` | `string` | The name or URL of the schema describing the resource (e.g., "Issue", "Comment"). |
| `resource_id` | `string` | The unique ID of the resource being modified. |
| `actor` | `string` | The user or system performing the change (e.g., an email address). |
| `timestamp` | `string` | ISO 8601 timestamp of the commit. |
| `resource_data`| `object` | (Optional) Complete resource data for creation or replacement. |
| `patch` | `object` | (Optional) JSON Merge Patch for incremental updates. |
| `deleted` | `boolean` | (Optional) Set to `true` to mark the resource as deleted. |

---

## HTTP Transport

### Sending Events (`POST /events`)

To commit a change, send a CloudEvent containing a `JSONCommit` to the `/events` endpoint.

```http
POST /events
Content-Type: application/json

{
  "specversion": "1.0",
  "id": "evt-12345",
  "type": "json.commit",
  "source": "frontend-app",
  "subject": "zaak-5",
  "data": {
    "schema": "Comment",
    "resource_id": "comment-789",
    "actor": "bob@gemeente.nl",
    "resource_data": {
      "content": "Status updated to in progress",
      "author": "bob@gemeente.nl"
    }
  }
}
```

### Receiving Events (`GET /events`)

Events can be consumed in real-time or via polling.

#### 1. Server-Sent Events (SSE) - Default
The default behavior of `GET /events` is to stream events using SSE.
- Upon connection, the server sends a `snapshot` event containing recent history.
- Subsequent changes are pushed as `delta` events.

#### 2. JSON Format
To retrieve a static list of events, use the `?format=json` query parameter.

---

## Querying and Filtering

| Parameter | Type | Description |
| :--- | :--- | :--- |
| `topic` | `string` | Filter events by subject or type (e.g., `?topic=zaak-5`). |
| `after_seq` | `string` | Get events occurring after a specific sequence number for pagination. |
| `limit` | `number` | Limit the number of events returned (default: 100). |
| `token` | `string` | JWT authentication token (required). |

---

## Merits of Event Sourcing

Using `JSONCommit` as the primary vehicle for state change facilitates robust **Event Sourcing**:

1.  **Immutable Audit Trail**: Every change is recorded as an immutable event. You can see *who* changed *what* and *when*.
2.  **State Reconstruction**: The current state of any resource can be reconstructed by playing back its sequence of `JSONCommit` events.
3.  **Materialized Views**: Different systems can build their own specialized views (e.g., a search index, a dashboard) by subscribing to the event stream.
4.  **Temporal Queries**: It becomes possible to query the state of the system as it existed at any point in time.

---

## Examples

#### Create a New Issue
```json
{
  "type": "json.commit",
  "subject": "zaak-2024-001",
  "data": {
    "schema": "Issue",
    "resource_id": "zaak-2024-001",
    "actor": "alice@gemeente.nl",
    "resource_data": {
      "title": "Broken streetlight",
      "status": "open",
      "involved": ["alice@gemeente.nl"]
    }
  }
}
```

#### Update Status (Patch)
```json
{
  "type": "json.commit",
  "subject": "zaak-2024-001",
  "data": {
    "schema": "Issue",
    "resource_id": "zaak-2024-001",
    "patch": {
      "status": "in_progress"
    }
  }
}
```

#### Delete a Task
```json
{
  "type": "json.commit",
  "subject": "zaak-2024-001",
  "data": {
    "schema": "Task",
    "resource_id": "task-abc",
    "deleted": true
  }
}
```
