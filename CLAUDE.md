# CLAUDE.md – fs-registry

## What is this?

FreeSynergy Registry — service capability registry.
Answers "What capabilities are available on this node right now?"
Services register capabilities on startup, deregister on shutdown.

## Rules

- Language in files: **English** (comments, code, variable names)
- Language in chat: **German**
- OOP everywhere: traits over match blocks, types carry their own behavior
- No CHANGELOG.md
- After every feature: commit directly

## Quality Gates (before every commit)

```
cargo clippy --all-targets -- -D warnings
cargo fmt --check
cargo test
cargo build --release
```

Every lib.rs / main.rs must have:
```rust
#![deny(clippy::all, clippy::pedantic, warnings)]
```

## Architecture

```
ServiceRegistry (trait)         — register / deregister / list / by_capability / endpoint_for / set_status
    ^
Registry                        — SQLite-backed impl (sea-orm, :memory: in tests)
    ^
GrpcRegistry                    — tonic RegistryService server (wraps Arc<dyn ServiceRegistry>)
REST router (axum)              — POST /services, DELETE /services/{id}, GET /capabilities/{cap}
CLI (clap)                      — fs-registry list / lookup {cap} / status
RegistryBusHandler              — fs-bus subscriber: service.started → register, service.stopped → deregister
```

## Ports

| Server | Default port |
|--------|-------------|
| gRPC   | 50060        |
| REST   | 8060         |

## Dependencies

- `sea-orm =2.0.0-rc.37` (SQLite via sqlx-sqlite)
- `tonic 0.12`, `prost 0.13`
- `axum 0.7`, `utoipa 5`, `utoipa-swagger-ui 8`
- `clap 4` (derive + env features)
- `fs-bus` (bus handler for service.# events)
