// service_registry.rs — ServiceRegistry trait (Registry Pattern).
//
// All concrete registry backends implement this trait.  Consumer code
// (gRPC server, REST server, CLI client, bus handler) only ever depends on
// `dyn ServiceRegistry` — never on the concrete `Registry` type.

use async_trait::async_trait;

use crate::{
    error::RegistryError,
    models::{ServiceEntry, ServiceStatus},
};

/// The primary interface for the service capability registry.
///
/// A service capability registry tracks *which services are currently running*
/// and *which capabilities each service provides*.  Callers query the registry
/// to find an endpoint for a required capability, avoiding hard-coded service
/// addresses.
///
/// # Design
///
/// This is a pure trait — the concrete implementation (`Registry`) is hidden
/// behind `Arc<dyn ServiceRegistry>` in all server and CLI code.
#[async_trait]
pub trait ServiceRegistry: Send + Sync {
    /// Register or update a service capability entry.
    ///
    /// If an entry with the same `id` already exists it is replaced, so
    /// calling `register` on startup is idempotent.
    ///
    /// # Errors
    /// Returns [`RegistryError`] on persistence failure.
    async fn register(&self, entry: ServiceEntry) -> Result<(), RegistryError>;

    /// Deregister all capabilities of `service_id`.
    ///
    /// Called when a service shuts down.  No-op if the service is not
    /// currently registered.
    ///
    /// # Errors
    /// Returns [`RegistryError`] on persistence failure.
    async fn deregister(&self, service_id: &str) -> Result<(), RegistryError>;

    /// All registered entries.
    ///
    /// # Errors
    /// Returns [`RegistryError`] on persistence failure.
    async fn list(&self) -> Result<Vec<ServiceEntry>, RegistryError>;

    /// All entries registered for `capability`.
    ///
    /// # Errors
    /// Returns [`RegistryError`] on persistence failure.
    async fn by_capability(&self, capability: &str) -> Result<Vec<ServiceEntry>, RegistryError>;

    /// All capabilities registered by `service_id`.
    ///
    /// # Errors
    /// Returns [`RegistryError`] on persistence failure.
    async fn by_service(&self, service_id: &str) -> Result<Vec<ServiceEntry>, RegistryError>;

    /// The endpoint of the first `Up` service for `capability`.
    ///
    /// Returns `None` if no service is registered for the capability or all
    /// registered services are currently `Down`.
    ///
    /// # Errors
    /// Returns [`RegistryError`] on persistence failure.
    async fn endpoint_for_capability(
        &self,
        capability: &str,
    ) -> Result<Option<String>, RegistryError>;

    /// Update the reachability status of a specific entry.
    ///
    /// # Errors
    /// Returns [`RegistryError::NotFound`] if no entry with `id` exists, or a
    /// persistence error.
    async fn set_status(&self, id: &str, status: ServiceStatus) -> Result<(), RegistryError>;
}
