#![deny(clippy::all, clippy::pedantic, warnings)]
//! `fs-registry` — service capability registry daemon for `FreeSynergy`.
//!
//! Starts a gRPC server (tonic) and a REST server (axum) and subscribes to
//! `registry::service::*` bus events to keep the registry in sync.
//!
//! On startup the daemon publishes `registry::service::registered` on the
//! local in-process bus so any co-located listeners know the registry is
//! ready.  In production a `BusBridge` can be added to the bus to forward
//! events from (and to) remote bus instances.
//!
//! # Environment variables
//!
//! | Variable              | Default                                   |
//! |-----------------------|-------------------------------------------|
//! | `FS_REGISTRY_DB`      | `/var/lib/freesynergy/registry.db`        |
//! | `FS_GRPC_PORT`        | `50060`                                   |
//! | `FS_REST_PORT`        | `8060`                                    |

use std::{net::SocketAddr, sync::Arc};

use clap::Parser as _;
use fs_bus::{
    message::BusMessage,
    topics::{REGISTRY_SERVICE_REGISTERED, REGISTRY_SERVICE_STOPPED},
    Event, MessageBus,
};
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

use fs_registry::{
    bus_handler::RegistryBusHandler,
    cli::Cli,
    grpc::{GrpcRegistry, RegistryServiceServer},
    registry::Registry,
    rest, ServiceStartedPayload, ServiceStoppedPayload,
};

// ── Config ────────────────────────────────────────────────────────────────────

struct Config {
    db_path: String,
    grpc_addr: SocketAddr,
    rest_addr: SocketAddr,
}

impl Config {
    fn from_env() -> Self {
        let db_path = std::env::var("FS_REGISTRY_DB")
            .unwrap_or_else(|_| "/var/lib/freesynergy/registry.db".into());
        let grpc_port: u16 = std::env::var("FS_GRPC_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(50_060);
        let rest_port: u16 = std::env::var("FS_REST_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8_060);
        Self {
            db_path,
            grpc_addr: format!("0.0.0.0:{grpc_port}")
                .parse()
                .expect("valid grpc addr"),
            rest_addr: format!("0.0.0.0:{rest_port}")
                .parse()
                .expect("valid rest addr"),
        }
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    // ── CLI mode ──────────────────────────────────────────────────────────────
    // If a subcommand is given the binary acts as a CLI client (connects to
    // the running daemon via gRPC).  Without arguments it starts the daemon.
    let args: Vec<String> = std::env::args().collect();
    let is_daemon = args.len() == 1 || args.get(1).is_some_and(|a| a.starts_with("--"));

    if !is_daemon {
        let cli = Cli::parse();
        return fs_registry::cli::run(cli)
            .await
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>);
    }

    // ── Daemon mode ───────────────────────────────────────────────────────────
    let cfg = Config::from_env();
    info!(
        db   = %cfg.db_path,
        grpc = %cfg.grpc_addr,
        rest = %cfg.rest_addr,
        "starting fs-registry daemon",
    );

    // Open registry — keep Arc<Registry> for the bus handler, then coerce to
    // the trait object used by gRPC and REST.
    let registry_arc = Arc::new(Registry::open(&cfg.db_path).await?);
    let registry: Arc<dyn fs_registry::service_registry::ServiceRegistry> =
        Arc::clone(&registry_arc) as Arc<dyn fs_registry::service_registry::ServiceRegistry>;

    // ── In-process bus ────────────────────────────────────────────────────────
    // Build the local bus and attach the RegistryBusHandler so that
    // registry::service::* events published on this bus are persisted.
    let mut bus = MessageBus::new();
    bus.add_handler(Arc::new(RegistryBusHandler::new(Arc::clone(&registry_arc))));
    let bus = Arc::new(bus);

    // Publish own startup event so co-located listeners know the registry is up.
    let grpc_endpoint = format!("http://{}", cfg.grpc_addr);
    let startup_ev = Event::new(
        REGISTRY_SERVICE_REGISTERED,
        "fs-registry",
        ServiceStartedPayload {
            service_id: "fs-registry".into(),
            capability: "registry".into(),
            endpoint: grpc_endpoint,
        },
    )?;
    bus.publish(BusMessage::fire(startup_ev)).await;
    info!("published registry::service::registered on local bus");

    // gRPC server.
    let grpc_registry = GrpcRegistry::new(Arc::clone(&registry));
    let grpc_svc = RegistryServiceServer::new(grpc_registry);
    let grpc_server = tonic::transport::Server::builder()
        .add_service(grpc_svc)
        .serve(cfg.grpc_addr);

    // REST server.
    let rest_router = rest::router(Arc::clone(&registry));
    let rest_listener = tokio::net::TcpListener::bind(cfg.rest_addr).await?;
    let rest_server = axum::serve(rest_listener, rest_router);

    info!("gRPC listening on {}", cfg.grpc_addr);
    info!("REST listening on {}", cfg.rest_addr);

    // Run both servers concurrently; stop on first error.
    tokio::select! {
        result = grpc_server => {
            if let Err(e) = result {
                warn!("gRPC server error: {e}");
            }
        }
        result = rest_server => {
            if let Err(e) = result {
                warn!("REST server error: {e}");
            }
        }
    }

    // Announce shutdown on the local bus.
    let stop_ev = Event::new(
        REGISTRY_SERVICE_STOPPED,
        "fs-registry",
        ServiceStoppedPayload {
            service_id: "fs-registry".into(),
        },
    )?;
    bus.publish(BusMessage::fire(stop_ev)).await;
    info!("published registry::service::stopped on local bus");

    Ok(())
}
