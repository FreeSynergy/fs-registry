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
```

Every lib.rs / main.rs must have:
```rust
#![deny(clippy::all, clippy::pedantic, warnings)]
```

## Architecture

- `Registry` — open database, register/deregister services, query by capability
- `ServiceEntry` — a registered service (name, capabilities, endpoint URL)
- `ServiceStatus` — `Active`, `Inactive`, `Degraded`

## Dependencies

- `sea-orm =2.0.0-rc.37` (SQLite)
