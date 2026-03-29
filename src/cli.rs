// cli.rs — fs-registry CLI (clap).
//
// Connects to the running registry daemon via gRPC and prints results.
//
// Commands:
//   fs-registry list                  — list all registered services
//   fs-registry lookup <capability>   — all services for a capability
//   fs-registry status                — health probe

use clap::{Parser, Subcommand};
use tracing::warn;

use crate::grpc::{
    proto::registry_service_client::RegistryServiceClient, EndpointForRequest, HealthRequest,
    ListRequest, LookupRequest,
};

// ── Args ──────────────────────────────────────────────────────────────────────

/// fs-registry — service capability registry CLI.
#[derive(Parser)]
#[command(name = "fs-registry", version, about)]
pub struct Cli {
    /// gRPC endpoint of the running registry daemon.
    #[arg(
        long,
        env = "FS_REGISTRY_GRPC",
        default_value = "http://127.0.0.1:50060"
    )]
    pub grpc: String,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List all registered service entries.
    List,
    /// Find all services registered for a capability.
    Lookup {
        /// Capability identifier, e.g. `iam` or `db.engine.sqlite`.
        capability: String,
    },
    /// Show the registry daemon health.
    Status,
}

// ── Runner ────────────────────────────────────────────────────────────────────

/// Execute a CLI command.
///
/// Connects to the registry gRPC endpoint and prints the result to stdout.
///
/// # Errors
///
/// Returns an error string if the gRPC connection or the command fails.
pub async fn run(cli: Cli) -> Result<(), String> {
    let mut client = RegistryServiceClient::connect(cli.grpc.clone())
        .await
        .map_err(|e| format!("cannot connect to {}: {e}", cli.grpc))?;

    match cli.command {
        Command::List => {
            let resp = client
                .list(ListRequest {})
                .await
                .map_err(|e| e.to_string())?
                .into_inner();

            if resp.entries.is_empty() {
                println!("No services registered.");
            } else {
                println!("{:<30} {:<25} {:<8} REGISTERED", "ID", "ENDPOINT", "STATUS");
                println!("{}", "-".repeat(90));
                for e in resp.entries {
                    println!(
                        "{:<30} {:<25} {:<8} {}",
                        e.id, e.endpoint, e.status, e.registered_at
                    );
                }
            }
        }

        Command::Lookup { capability } => {
            let resp = client
                .lookup(LookupRequest {
                    capability: capability.clone(),
                })
                .await
                .map_err(|e| e.to_string())?
                .into_inner();

            if resp.entries.is_empty() {
                println!("No services registered for capability '{capability}'.");
            } else {
                // Also show the active endpoint.
                let ep_resp = client
                    .endpoint_for(EndpointForRequest {
                        capability: capability.clone(),
                    })
                    .await
                    .map_err(|e| {
                        warn!("endpoint_for failed: {e}");
                        e.to_string()
                    })
                    .ok()
                    .map(tonic::Response::into_inner);

                if let Some(ep) = ep_resp.filter(|r| r.found) {
                    println!("Active endpoint for '{capability}': {}", ep.endpoint);
                } else {
                    println!("No active endpoint for '{capability}'.");
                }
                println!();
                println!("{:<30} {:<25} STATUS", "ID", "ENDPOINT");
                println!("{}", "-".repeat(70));
                for e in resp.entries {
                    println!("{:<30} {:<25} {}", e.id, e.endpoint, e.status);
                }
            }
        }

        Command::Status => {
            let resp = client
                .health(HealthRequest {})
                .await
                .map_err(|e| e.to_string())?
                .into_inner();

            if resp.ok {
                println!("Registry: OK ({} entries)", resp.entry_count);
            } else {
                println!("Registry: DEGRADED — {}", resp.message);
            }
        }
    }

    Ok(())
}
