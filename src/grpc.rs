// grpc.rs — gRPC service implementation for fs-registry.
//
// Wraps `Arc<dyn ServiceRegistry>` and exposes it via the RegistryService proto.
// Routes:
//   Register / Deregister / List / Lookup / EndpointFor / SetStatus / Health

use std::sync::Arc;

use tonic::{Request, Response, Status};
use tracing::instrument;

use crate::{models::ServiceEntry, service_registry::ServiceRegistry};

// Include the generated tonic code.
pub mod proto {
    #![allow(clippy::all, clippy::pedantic, warnings)]
    tonic::include_proto!("registry");
}

pub use proto::registry_service_server::{RegistryService, RegistryServiceServer};
pub use proto::{
    DeregisterRequest, DeregisterResponse, EndpointForRequest, EndpointForResponse, HealthRequest,
    HealthResponse, ListRequest, ListResponse, LookupRequest, LookupResponse, RegisterRequest,
    RegisterResponse, ServiceEntryProto, SetStatusRequest, SetStatusResponse,
};

// ── Conversions ───────────────────────────────────────────────────────────────

fn entry_to_proto(e: ServiceEntry) -> ServiceEntryProto {
    ServiceEntryProto {
        id: e.id,
        service_id: e.service_id,
        capability: e.capability,
        endpoint: e.endpoint,
        status: e.status.to_string().to_lowercase(),
        registered_at: e.registered_at.to_rfc3339(),
    }
}

fn parse_status(s: &str) -> crate::models::ServiceStatus {
    match s {
        "up" => crate::models::ServiceStatus::Up,
        "down" => crate::models::ServiceStatus::Down,
        _ => crate::models::ServiceStatus::Unknown,
    }
}

// ── GrpcRegistry ─────────────────────────────────────────────────────────────

/// gRPC service wrapper around a shared [`ServiceRegistry`].
pub struct GrpcRegistry {
    registry: Arc<dyn ServiceRegistry>,
}

impl GrpcRegistry {
    /// Wrap `registry` in a gRPC service.
    #[must_use]
    pub fn new(registry: Arc<dyn ServiceRegistry>) -> Self {
        Self { registry }
    }
}

#[tonic::async_trait]
impl RegistryService for GrpcRegistry {
    #[instrument(name = "grpc.register", skip(self, request))]
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<Response<RegisterResponse>, Status> {
        let req = request.into_inner();
        let entry = ServiceEntry::new(&req.service_id, &req.capability, &req.endpoint);
        match self.registry.register(entry).await {
            Ok(()) => Ok(Response::new(RegisterResponse {
                ok: true,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(RegisterResponse {
                ok: false,
                message: e.to_string(),
            })),
        }
    }

    #[instrument(name = "grpc.deregister", skip(self, request))]
    async fn deregister(
        &self,
        request: Request<DeregisterRequest>,
    ) -> Result<Response<DeregisterResponse>, Status> {
        let req = request.into_inner();
        match self.registry.deregister(&req.service_id).await {
            Ok(()) => Ok(Response::new(DeregisterResponse {
                ok: true,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(DeregisterResponse {
                ok: false,
                message: e.to_string(),
            })),
        }
    }

    #[instrument(name = "grpc.list", skip(self, _request))]
    async fn list(&self, _request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        match self.registry.list().await {
            Ok(entries) => Ok(Response::new(ListResponse {
                entries: entries.into_iter().map(entry_to_proto).collect(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    #[instrument(name = "grpc.lookup", skip(self, request))]
    async fn lookup(
        &self,
        request: Request<LookupRequest>,
    ) -> Result<Response<LookupResponse>, Status> {
        let capability = request.into_inner().capability;
        match self.registry.by_capability(&capability).await {
            Ok(entries) => Ok(Response::new(LookupResponse {
                entries: entries.into_iter().map(entry_to_proto).collect(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    #[instrument(name = "grpc.endpoint_for", skip(self, request))]
    async fn endpoint_for(
        &self,
        request: Request<EndpointForRequest>,
    ) -> Result<Response<EndpointForResponse>, Status> {
        let capability = request.into_inner().capability;
        match self.registry.endpoint_for_capability(&capability).await {
            Ok(Some(endpoint)) => Ok(Response::new(EndpointForResponse {
                found: true,
                endpoint,
            })),
            Ok(None) => Ok(Response::new(EndpointForResponse {
                found: false,
                endpoint: String::new(),
            })),
            Err(e) => Err(Status::internal(e.to_string())),
        }
    }

    #[instrument(name = "grpc.set_status", skip(self, request))]
    async fn set_status(
        &self,
        request: Request<SetStatusRequest>,
    ) -> Result<Response<SetStatusResponse>, Status> {
        let req = request.into_inner();
        let status = parse_status(&req.status);
        match self.registry.set_status(&req.id, status).await {
            Ok(()) => Ok(Response::new(SetStatusResponse {
                ok: true,
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(SetStatusResponse {
                ok: false,
                message: e.to_string(),
            })),
        }
    }

    #[instrument(name = "grpc.health", skip(self, _request))]
    async fn health(
        &self,
        _request: Request<HealthRequest>,
    ) -> Result<Response<HealthResponse>, Status> {
        match self.registry.list().await {
            Ok(entries) => Ok(Response::new(HealthResponse {
                ok: true,
                entry_count: u32::try_from(entries.len()).unwrap_or(u32::MAX),
                message: String::new(),
            })),
            Err(e) => Ok(Response::new(HealthResponse {
                ok: false,
                entry_count: 0,
                message: e.to_string(),
            })),
        }
    }
}
