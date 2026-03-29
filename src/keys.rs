// keys.rs — FTL key name constants for fs-registry.
//
// All user-visible strings in this crate are translated via fs-i18n.
// The matching .ftl files live at:
//   fs-i18n/locales/{lang}/registry.ftl

// ── Status ────────────────────────────────────────────────────────────────────

/// CLI listing header.
pub const CLI_LISTING_HEADER: &str = "registry-cli-listing-header";

/// Shown when no services are registered.
pub const CLI_NO_SERVICES: &str = "registry-cli-no-services";

/// Endpoint lookup result (variables: `capability`, `endpoint`).
pub const CLI_ENDPOINT_FOUND: &str = "registry-cli-endpoint-found";

/// Endpoint not found for capability (variable: `capability`).
pub const CLI_ENDPOINT_NOT_FOUND: &str = "registry-cli-endpoint-not-found";

// ── Errors ────────────────────────────────────────────────────────────────────

/// Database error (variable: `reason`).
pub const ERROR_DATABASE: &str = "registry-error-database";

/// Entry not found (variable: `id`).
pub const ERROR_NOT_FOUND: &str = "registry-error-not-found";

/// JSON serialisation error (variable: `reason`).
pub const ERROR_JSON: &str = "registry-error-json";

// ── Info ──────────────────────────────────────────────────────────────────────

/// Service registered (variables: `name`, `capability`).
pub const INFO_REGISTERED: &str = "registry-info-registered";

/// Service unregistered (variable: `name`).
pub const INFO_UNREGISTERED: &str = "registry-info-unregistered";

/// Server started (variables: `grpc`, `rest`).
pub const INFO_SERVER_STARTED: &str = "registry-info-server-started";
