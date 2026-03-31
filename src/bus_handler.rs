// bus_handler.rs — RegistryBusHandler: bridges fs-bus registry::service::* events to
// the Registry database.
//
// Topic patterns handled:
//   registry::service::registered  → registry.register(entry)
//   registry::service::stopped     → registry.deregister(service_id)
//   registry::capability::added    → (logged, no extra action needed)
//   registry::capability::removed  → (logged, no extra action needed)
//
// Unknown topics and malformed payloads are logged and not propagated so a
// single bad message cannot disrupt the rest of the bus.

use std::sync::Arc;

use async_trait::async_trait;
use fs_bus::topics::{
    REGISTRY_CAPABILITY_ADDED, REGISTRY_CAPABILITY_REMOVED, REGISTRY_SERVICE_REGISTERED,
    REGISTRY_SERVICE_STOPPED,
};
use fs_bus::{BusError, Event, TopicHandler};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument, warn};

use crate::{models::ServiceEntry, registry::Registry};

// ── Payload types ─────────────────────────────────────────────────────────────

/// Payload of `service.started`.
#[derive(Debug, Deserialize, Serialize)]
pub struct ServiceStartedPayload {
    /// Service identifier, e.g. `"kanidm"`.
    pub service_id: String,
    /// Capability this service provides, e.g. `"iam"`.
    pub capability: String,
    /// Base URL of the service's HTTP API, e.g. `"http://kanidm:8443"`.
    pub endpoint: String,
}

/// Payload of `service.stopped`.
#[derive(Debug, Deserialize, Serialize)]
pub struct ServiceStoppedPayload {
    pub service_id: String,
}

// ── RegistryBusHandler ────────────────────────────────────────────────────────

/// Subscribes to `service.#` bus events and keeps `fs-registry.db` in sync.
pub struct RegistryBusHandler {
    registry: Arc<Registry>,
}

impl RegistryBusHandler {
    /// Wrap an existing `Registry` in a bus handler.
    #[must_use]
    pub fn new(registry: Arc<Registry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl TopicHandler for RegistryBusHandler {
    fn topic_pattern(&self) -> &'static str {
        "registry::*"
    }

    #[instrument(name = "registry.bus_handler", skip(self, event), fields(topic = event.topic()))]
    async fn handle(&self, event: &Event) -> Result<(), BusError> {
        match event.topic() {
            REGISTRY_SERVICE_REGISTERED => {
                let payload: ServiceStartedPayload = match event.parse_payload() {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("registry::service::registered: bad payload: {e}");
                        return Ok(());
                    }
                };
                let entry =
                    ServiceEntry::new(&payload.service_id, &payload.capability, &payload.endpoint);
                if let Err(e) = self.registry.register(entry).await {
                    warn!("registry register failed: {e}");
                }
            }
            REGISTRY_SERVICE_STOPPED => {
                let payload: ServiceStoppedPayload = match event.parse_payload() {
                    Ok(p) => p,
                    Err(e) => {
                        warn!("registry::service::stopped: bad payload: {e}");
                        return Ok(());
                    }
                };
                if let Err(e) = self.registry.deregister(&payload.service_id).await {
                    warn!("registry deregister failed: {e}");
                }
            }
            REGISTRY_CAPABILITY_ADDED | REGISTRY_CAPABILITY_REMOVED => {
                info!("registry capability event: {}", event.topic());
            }
            other => {
                warn!("RegistryBusHandler: unhandled topic '{other}'");
            }
        }
        Ok(())
    }
}
