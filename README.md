# fs-registry

Service capability registry for FreeSynergy — answers
"What capabilities are available on this node right now?"

## Build

```sh
cargo build --release
cargo test
```

## Architecture

- `Registry` — open database, register/deregister services, query by capability
- `ServiceEntry` — a registered service (name, capabilities, endpoint URL)
- `ServiceStatus` — `Active`, `Inactive`, `Degraded`

Services register their capabilities on startup and deregister on shutdown.
The Bus routes requests to whoever is registered for the required capability.
Replaces the old Bridge concept with a standard trait + registry pattern.
