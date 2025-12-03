use zaakchat::search::SearchIndex;
use zaakchat::{handlers, schemas};
pub mod auth;

use futures_util::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

use axum::{
    extract::State,
    http::StatusCode,
    response::sse::{Event, KeepAlive, Sse},
    response::{Html, Response},
    routing::{delete, get, post},
    serve, Json, Router,
};
use std::{convert::Infallible, sync::Arc};
use tokio::sync::{broadcast, RwLock};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;
use tower_http::services::ServeDir;
use tower_http::{cors::CorsLayer, services::ServeFile};

use zaakchat::storage::Storage;

#[derive(Clone)]
pub struct AppState {
    // Storage layer (persistent DB)
    pub storage: Arc<Storage>,
    // Search index manager (separate module handling Tantivy)
    pub search: Arc<SearchIndex>,
    // Broadcast deltas to all subscribers
    pub tx: broadcast::Sender<schemas::CloudEvent>,
    // Base URL for generating schema URLs
    #[allow(dead_code)]
    pub base_url: String,
    // Push notification subscriptions
    pub push_subscriptions: Arc<RwLock<Vec<zaakchat::PushSubscription>>>,
}

/// CloudEvent following the CloudEvents specification v1.0
pub use schemas::CloudEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct IncomingCloudEvent {
    specversion: String,
    id: String,
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject: Option<String>,
    #[serde(rename = "type")]
    event_type: String,
    time: Option<String>,
    datacontenttype: Option<String>,
    data: Option<Value>,
}

#[tokio::main]
async fn main() {
    let app = create_app().await;
    let addr = "0.0.0.0:8000";
    println!("→ http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    serve(listener, app).await.unwrap();
}

async fn create_app() -> Router {
    if !std::path::Path::new("dist").exists() {
        panic!("Frontend dist folder is missing! Please build the frontend first with: cd frontend && pnpm run build");
    }

    // Get base URL from environment or use default
    let base_url =
        std::env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());

    // Initialize storage
    let data_dir = std::env::var("DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./data"));

    let storage = Storage::new(&data_dir)
        .await
        .expect("Failed to initialize storage");

    // Initialize search index (separate module)
    let search_index = match zaakchat::search::SearchIndex::open(
        &data_dir.join("search_index"),
        true,
        std::time::Duration::from_secs(10),
    ) {
        Ok(si) => Arc::new(si),
        Err(e) => panic!("Failed to initialize search index: {}", e),
    };

    let (tx, _) = broadcast::channel(256);

    let state = AppState {
        storage: Arc::new(storage),
        search: search_index.clone(),
        tx: tx.clone(),
        base_url: base_url.clone(),
        push_subscriptions: Arc::new(RwLock::new(Vec::new())),
    };

    // Create handler state
    let handler_state = handlers::AppState {
        storage: state.storage.clone(),
        search: state.search.clone(),
        tx: state.tx.clone(),
        push_subscriptions: state.push_subscriptions.clone(),
    };

    // API routes with new storage-backed endpoints
    let api_routes = Router::new()
        // SSE endpoint for real-time updates (kept for backward compatibility)
        .route("/events/stream", get(sse_handler))
        // Command + Sync endpoint: GET will stream SSE when the client requests
        // `Accept: text/event-stream`, otherwise it returns a JSON list.
        .route(
            "/events",
            get(handlers::get_or_stream_events).post(handlers::handle_event),
        )
        // Resource endpoints
        .route("/resources", get(handlers::list_resources))
        .route("/resources/{id}", get(handlers::get_resource))
        .route("/resources/{id}", delete(handlers::delete_resource))
        // Query endpoint with Tantivy search
        .route("/query", get(handlers::query_resources))
        // Debug endpoint to inspect persisted DB counts and samples
        .route("/debug/db", get(handlers::debug_db))
        // Legacy endpoints (can be removed later)
        .route("/reset/", post(reset_state_handler))
        .route("/schemas", get(crate::schemas::handle_get_schemas_index))
        .route("/schemas/{*name}", get(crate::schemas::handle_get_schema))
        .route("/login", post(handlers::login_handler))
        .with_state(handler_state);

    // Combine API routes with static file serving
    let app = Router::new()
        .merge(api_routes)
        .route("/asyncapi-docs/asyncapi.yaml", get(serve_asyncapi_yaml))
        .route("/asyncapi-docs/asyncapi.json", get(serve_asyncapi_json))
        .route("/asyncapi-docs", get(serve_asyncapi_docs))
        .nest_service("/asyncapi-docs/css", ServeDir::new("asyncapi-docs/css"))
        .nest_service("/asyncapi-docs/js", ServeDir::new("asyncapi-docs/js"))
        .fallback_service(ServeDir::new("dist").fallback(ServeFile::new("dist/index.html")))
        .layer(CorsLayer::permissive());

    app
}



/* The helper `extract_resource_type` was removed from `main.rs` because resource-type
detection is handled centrally in the handlers module. Keeping duplicate helpers
here caused unused-function warnings. If a shared helper is desired in future,
move it to a single common module (e.g., `handlers` or `types`) and import it where needed. */

/// SSE handler for streaming events
async fn sse_handler(
    State(state): State<handlers::AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.tx.subscribe();

    // Get snapshot from storage
    let snapshot_events = state.storage.list_events(0, 1000).await.unwrap_or_default();

    let snapshot = serde_json::to_string(&snapshot_events).unwrap();

    let stream = stream::once(async move { Ok(Event::default().event("snapshot").data(snapshot)) })
        .chain(
            BroadcastStream::new(rx)
                .map(|msg| {
                    let delta = msg.unwrap();
                    let json = serde_json::to_string(&delta).unwrap();
                    Event::default().event("delta").data(json)
                })
                .map(Ok),
        );

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Reset state handler
async fn reset_state_handler(
    State(state): State<handlers::AppState>,
) -> Result<Json<&'static str>, StatusCode> {
    // Clear storage
    if let Err(e) = state.storage.clear().await {
        eprintln!("Failed to clear storage: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Clear search index
    if let Err(e) = state.search.clear().await {
        eprintln!("Failed to clear search index: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json("ok"))
}

/// Push subscription handler
#[allow(dead_code)]
// `subscribe_push` local stub removed: routes now use `crate::push::subscribe_push`

/// Push unsubscribe handler
#[allow(dead_code)]
// `unsubscribe_push` local stub removed: routes now use `crate::push::unsubscribe_push`

/// Serve the AsyncAPI HTML documentation
async fn serve_asyncapi_docs() -> Result<Html<String>, StatusCode> {
    let docs_path = std::path::Path::new("asyncapi-docs/index.html");
    if !docs_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    match tokio::fs::read_to_string(docs_path).await {
        Ok(content) => Ok(Html(content)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Serve the AsyncAPI YAML file
async fn serve_asyncapi_yaml() -> Result<Response, StatusCode> {
    let yaml_path = std::path::Path::new("asyncapi.yaml");
    if !yaml_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    match tokio::fs::read_to_string(yaml_path).await {
        Ok(content) => {
            let mut response = Response::new(content.into());
            response.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("application/x-yaml"),
            );
            Ok(response)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Serve the AsyncAPI JSON file
async fn serve_asyncapi_json() -> Result<Response, StatusCode> {
    let json_path = std::path::Path::new("asyncapi.json");
    if !json_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    match tokio::fs::read_to_string(json_path).await {
        Ok(content) => {
            let mut response = Response::new(content.into());
            response.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                axum::http::HeaderValue::from_static("application/json"),
            );
            Ok(response)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
