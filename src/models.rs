//! Domain models for the service registry.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

// ── ServiceStatus ─────────────────────────────────────────────────────────────

/// Reachability status of a registered service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    #[default]
    Up,
    Down,
    Unknown,
}

impl fmt::Display for ServiceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Up => write!(f, "Up"),
            Self::Down => write!(f, "Down"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

// ── ServiceEntry ──────────────────────────────────────────────────────────────

/// A service that has registered one of its capabilities with the registry.
///
/// One entry per (service, capability) pair. A single service can register
/// multiple capabilities (e.g. `"iam"` and `"scim"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEntry {
    /// Unique entry id: `"{service_id}::{capability}"`, e.g. `"kanidm::iam"`.
    pub id: String,
    /// Service name, e.g. `"kanidm"`.
    pub service_id: String,
    /// Capability this entry advertises, e.g. `"iam"`.
    pub capability: String,
    /// Base URL of the service endpoint, e.g. `"http://kanidm:8443"`.
    pub endpoint: String,
    /// Current reachability status.
    pub status: ServiceStatus,
    /// When this entry was registered.
    pub registered_at: DateTime<Utc>,
}

impl ServiceEntry {
    /// Create a new entry with `ServiceStatus::Up`.
    pub fn new(
        service_id: impl Into<String>,
        capability: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        let service_id = service_id.into();
        let capability = capability.into();
        Self {
            id: format!("{service_id}::{capability}"),
            service_id,
            capability,
            endpoint: endpoint.into(),
            status: ServiceStatus::Up,
            registered_at: Utc::now(),
        }
    }

    #[must_use]
    pub fn is_up(&self) -> bool {
        self.status == ServiceStatus::Up
    }
}
