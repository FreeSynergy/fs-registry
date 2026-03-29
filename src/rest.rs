// triggered by utoipa's OpenApi derive macro
#![allow(clippy::needless_for_each)]
// rest.rs — REST API for fs-registry (axum + utoipa OpenAPI).
//
// Routes:
//   POST   /services                    — register a service capability
//   DELETE /services/{service_id}       — deregister all capabilities of a service
//   GET    /services                    — list all registered entries
//   GET    /capabilities/{cap}          — all entries for a capability
//   GET    /capabilities/{cap}/endpoint — first Up endpoint for a capability
//   PUT    /services/{id}/status        — update entry status
//   GET    /health                      — health probe

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

use crate::{models::ServiceEntry, service_registry::ServiceRegistry};

// ── Shared state ──────────────────────────────────────────────────────────────

pub type SharedRegistry = Arc<dyn ServiceRegistry>;

// ── Request / Response types ─────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterBody {
    pub service_id: String,
    pub capability: String,
    pub endpoint: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OkResponse {
    pub ok: bool,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ServiceEntryResponse {
    pub id: String,
    pub service_id: String,
    pub capability: String,
    pub endpoint: String,
    pub status: String,
    pub registered_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EndpointResponse {
    pub found: bool,
    pub endpoint: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub ok: bool,
    pub entry_count: usize,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetStatusBody {
    pub status: String, // "up" | "down" | "unknown"
}

impl From<ServiceEntry> for ServiceEntryResponse {
    fn from(e: ServiceEntry) -> Self {
        Self {
            id: e.id,
            service_id: e.service_id,
            capability: e.capability,
            endpoint: e.endpoint,
            status: e.status.to_string().to_lowercase(),
            registered_at: e.registered_at.to_rfc3339(),
        }
    }
}

fn parse_status(s: &str) -> crate::models::ServiceStatus {
    match s {
        "up" => crate::models::ServiceStatus::Up,
        "down" => crate::models::ServiceStatus::Down,
        _ => crate::models::ServiceStatus::Unknown,
    }
}

// ── OpenAPI spec ─────────────────────────────────────────────────────────────

#[derive(OpenApi)]
#[openapi(
    paths(
        register,
        deregister,
        list_services,
        lookup_capability,
        endpoint_for,
        set_status,
        health
    ),
    components(schemas(
        RegisterBody,
        OkResponse,
        ServiceEntryResponse,
        EndpointResponse,
        HealthResponse,
        SetStatusBody,
    )),
    tags((name = "fs-registry", description = "FreeSynergy Service Registry API"))
)]
pub struct ApiDoc;

// ── Router ────────────────────────────────────────────────────────────────────

/// Build the axum router with all REST routes and Swagger UI.
pub fn router(registry: SharedRegistry) -> Router {
    Router::new()
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .route("/services", post(register))
        .route("/services", get(list_services))
        .route("/services/{service_id}", delete(deregister))
        .route("/services/{id}/status", put(set_status))
        .route("/capabilities/{cap}", get(lookup_capability))
        .route("/capabilities/{cap}/endpoint", get(endpoint_for))
        .route("/health", get(health))
        .with_state(registry)
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// Register a service capability.
#[utoipa::path(
    post,
    path = "/services",
    request_body = RegisterBody,
    responses(
        (status = 201, description = "Registered", body = OkResponse),
        (status = 500, description = "Database error", body = OkResponse),
    )
)]
async fn register(
    State(registry): State<SharedRegistry>,
    Json(body): Json<RegisterBody>,
) -> impl IntoResponse {
    let entry = ServiceEntry::new(&body.service_id, &body.capability, &body.endpoint);
    match registry.register(entry).await {
        Ok(()) => (
            StatusCode::CREATED,
            Json(OkResponse {
                ok: true,
                message: String::new(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OkResponse {
                ok: false,
                message: e.to_string(),
            }),
        ),
    }
}

/// Deregister all capabilities of a service.
#[utoipa::path(
    delete,
    path = "/services/{service_id}",
    params(("service_id" = String, Path, description = "Service identifier")),
    responses(
        (status = 200, description = "Deregistered", body = OkResponse),
        (status = 500, description = "Database error", body = OkResponse),
    )
)]
async fn deregister(
    State(registry): State<SharedRegistry>,
    Path(service_id): Path<String>,
) -> impl IntoResponse {
    match registry.deregister(&service_id).await {
        Ok(()) => (
            StatusCode::OK,
            Json(OkResponse {
                ok: true,
                message: String::new(),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OkResponse {
                ok: false,
                message: e.to_string(),
            }),
        ),
    }
}

/// List all registered service entries.
#[utoipa::path(
    get,
    path = "/services",
    responses(
        (status = 200, description = "All entries", body = Vec<ServiceEntryResponse>),
        (status = 500, description = "Database error"),
    )
)]
async fn list_services(
    State(registry): State<SharedRegistry>,
) -> Result<Json<Vec<ServiceEntryResponse>>, StatusCode> {
    registry
        .list()
        .await
        .map(|v| Json(v.into_iter().map(Into::into).collect()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// All entries for a specific capability.
#[utoipa::path(
    get,
    path = "/capabilities/{cap}",
    params(("cap" = String, Path, description = "Capability identifier")),
    responses(
        (status = 200, description = "Matching entries", body = Vec<ServiceEntryResponse>),
        (status = 500, description = "Database error"),
    )
)]
async fn lookup_capability(
    State(registry): State<SharedRegistry>,
    Path(cap): Path<String>,
) -> Result<Json<Vec<ServiceEntryResponse>>, StatusCode> {
    registry
        .by_capability(&cap)
        .await
        .map(|v| Json(v.into_iter().map(Into::into).collect()))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// First Up endpoint for a capability.
#[utoipa::path(
    get,
    path = "/capabilities/{cap}/endpoint",
    params(("cap" = String, Path, description = "Capability identifier")),
    responses(
        (status = 200, description = "Endpoint lookup result", body = EndpointResponse),
        (status = 500, description = "Database error"),
    )
)]
async fn endpoint_for(
    State(registry): State<SharedRegistry>,
    Path(cap): Path<String>,
) -> Result<Json<EndpointResponse>, StatusCode> {
    registry
        .endpoint_for_capability(&cap)
        .await
        .map(|ep| {
            Json(EndpointResponse {
                found: ep.is_some(),
                endpoint: ep,
            })
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Update the reachability status of an entry.
#[utoipa::path(
    put,
    path = "/services/{id}/status",
    params(("id" = String, Path, description = "Entry id (e.g. kanidm::iam)")),
    request_body = SetStatusBody,
    responses(
        (status = 200, description = "Status updated", body = OkResponse),
        (status = 404, description = "Entry not found", body = OkResponse),
        (status = 500, description = "Database error", body = OkResponse),
    )
)]
async fn set_status(
    State(registry): State<SharedRegistry>,
    Path(id): Path<String>,
    Json(body): Json<SetStatusBody>,
) -> impl IntoResponse {
    let status = parse_status(&body.status);
    match registry.set_status(&id, status).await {
        Ok(()) => (
            StatusCode::OK,
            Json(OkResponse {
                ok: true,
                message: String::new(),
            }),
        ),
        Err(crate::error::RegistryError::NotFound { .. }) => (
            StatusCode::NOT_FOUND,
            Json(OkResponse {
                ok: false,
                message: format!("entry '{id}' not found"),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(OkResponse {
                ok: false,
                message: e.to_string(),
            }),
        ),
    }
}

/// Health probe.
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Registry is healthy", body = HealthResponse),
        (status = 500, description = "Registry unavailable"),
    )
)]
async fn health(
    State(registry): State<SharedRegistry>,
) -> Result<Json<HealthResponse>, StatusCode> {
    registry
        .list()
        .await
        .map(|v| {
            Json(HealthResponse {
                ok: true,
                entry_count: v.len(),
            })
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
