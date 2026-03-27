//! `fs-registry` — service capability registry for `FreeSynergy`.
//!
//! The Registry answers the question *"What capabilities are available on this node right now?"*.
//!
//! A **capability** is a named ability a service advertises (e.g. `"iam"`, `"mail"`, `"storage"`).
//! Services register their capabilities on startup and deregister on shutdown.
//! Callers query by capability to find the endpoint that can handle a request.
//!
//! This replaces the old Bridge concept: instead of a dynamic bridge executor,
//! services implement a standard trait and register themselves here. The Bus
//! routes to whoever is registered for the required capability.
//!
//! # Example
//!
//! ```no_run
//! use fs_registry::{Registry, RegistryError, ServiceEntry, ServiceStatus};
//!
//! # async fn example() -> Result<(), RegistryError> {
//! let registry = Registry::open(":memory:").await?;
//!
//! let entry = ServiceEntry::new("kanidm", "iam", "http://kanidm:8443");
//! registry.register(entry).await?;
//!
//! let iam_services = registry.by_capability("iam").await?;
//! assert_eq!(iam_services.len(), 1);
//! # Ok(())
//! # }
//! ```

#![deny(clippy::all, clippy::pedantic, warnings)]
#![allow(clippy::module_name_repetitions)]

pub mod bus_handler;
pub mod error;
pub mod models;
pub mod registry;

pub use bus_handler::{RegistryBusHandler, ServiceStartedPayload, ServiceStoppedPayload};
pub use error::RegistryError;
pub use models::{ServiceEntry, ServiceStatus};
pub use registry::Registry;
