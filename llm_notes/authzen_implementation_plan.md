# Implementation Plan: AuthZEN Authorization

This document outlines the plan to migrate ZaakChat's authorization from hardcoded logic to the **AuthZEN** standard using **Topaz** as the Policy Decision Point (PDP).

## Objective
Externalize authorization logic to allow for flexible, role-based, and attribute-based access control (RBAC/ABAC) without modifying the Rust backend, while maintaining sub-millisecond performance.

## Technology Stack
- **PDP**: [Topaz](https://www.topaz.sh/) (AuthZEN-native, OPA-based sidecar).
- **Standard**: [OpenID AuthZEN Evaluation API](https://openid.net/wg/authzen/).
- **PEP**: Rust Axum middleware and handlers.
- **Search**: Tantivy (leveraging authorized query filtering).

---

## Phase 1: AuthZEN Client Foundation
1.  **Dependencies**: Add `reqwest` and `moka` (or use existing `dashmap`) for HTTP communication and caching.
2.  **Models**: Create `src/authzen.rs` defining the standardized JSON structures:
    - `Subject`: (ID + Attributes matching JWT claims).
    - `Resource`: (ID + Type + Data).
    - `Action`: (Name: "read", "write", "delete").
    - `EvaluationRequest` / `EvaluationResponse`.
3.  **Client**: Implement `AuthZenClient` in Rust to communicate with the Topaz sidecar (`/access/v1/evaluation`).

## Phase 2: Performance - The "Filter Strategy"
To avoid $O(N)$ checks for list operations (SSE snapshots and Search), we will implement **Authorization Filtering**:
1.  **Policy Residue**: Configure Topaz to return a "context" containing a list of authorized IDs or a query fragment for the current user.
2.  **Tantivy Integration**: Refactor `handlers::query_resources` to:
    - Ask Topaz: *"What filter applies to this user for Issues?"*
    - Receive: `involved:user_id` (or more complex logic like `dept:finance`).
    - Append this filter to the Tantivy search query.
3.  **SSE Snapshot**: Use the same filter logic to retrieve the localized snapshot.

## Phase 3: Integrity - Write Authorization
Currently, `POST /events` is unauthenticated. We will:
1.  **Secure Entry**: Update `handle_event` to require a valid `AuthUser`.
2.  **Point Check**: Perform a single AuthZEN evaluation: *"Can this user write to this specific Resource/Topic?"*
3.  **Audit Logs**: Ensure Topaz captures the decision for compliance.

## Phase 4: Optimization & Robustness
1.  **Sub-millisecond Decisions**: Deploy Topaz as a container sidecar or local process to eliminate network latency.
2.  **Result Caching**: Implement a short-lived LRU cache in `AuthZenClient` to eliminate redundant checks for the same Subject/Resource pair within the same session.
3.  **Fail-Closed**: Ensure the application defaults to `Deny` if the PDP is unreachable.

---

## Success Criteria
- [ ] Users can only search/see issues where Topaz grants `Permit`.
- [ ] Changing a policy in Topaz (e.g., "Allow all admins") takes effect without a server restart.
- [ ] Search latency overhead from AuthZEN is < 5ms.
