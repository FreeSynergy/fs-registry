//! `Registry` — the primary interface to `fs-registry.db`.

use crate::{
    error::RegistryError,
    models::{ServiceEntry, ServiceStatus},
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, Database, DatabaseConnection,
    EntityTrait, QueryFilter,
};
use tracing::instrument;

// ── Schema ────────────────────────────────────────────────────────────────────

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS service_entries (
    id            TEXT PRIMARY KEY NOT NULL,
    service_id    TEXT NOT NULL,
    capability    TEXT NOT NULL,
    endpoint      TEXT NOT NULL,
    status        TEXT NOT NULL DEFAULT 'up',
    registered_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_se_capability ON service_entries(capability);
CREATE INDEX IF NOT EXISTS idx_se_service    ON service_entries(service_id);
";

// ── SeaORM entity ─────────────────────────────────────────────────────────────

mod entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "service_entries")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub service_id: String,
        pub capability: String,
        pub endpoint: String,
        pub status: String,
        pub registered_at: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

// ── Conversion ────────────────────────────────────────────────────────────────

impl TryFrom<entity::Model> for ServiceEntry {
    type Error = RegistryError;

    fn try_from(m: entity::Model) -> Result<Self, Self::Error> {
        use chrono::DateTime;
        Ok(Self {
            id: m.id,
            service_id: m.service_id,
            capability: m.capability,
            endpoint: m.endpoint,
            status: serde_json::from_str(&format!("\"{}\"", m.status))?,
            registered_at: m
                .registered_at
                .parse::<DateTime<chrono::Utc>>()
                .unwrap_or_default(),
        })
    }
}

// ── Registry ──────────────────────────────────────────────────────────────────

/// The service capability registry.
#[derive(Debug)]
pub struct Registry {
    db: DatabaseConnection,
}

impl Registry {
    /// Open (or create) the registry database. Use `":memory:"` in tests.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] if the database connection fails or the schema cannot be applied.
    #[instrument(name = "registry.open")]
    pub async fn open(path: &str) -> Result<Self, RegistryError> {
        let url = if path == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite://{path}?mode=rwc")
        };
        let db = Database::connect(&url).await?;
        db.execute_unprepared(SCHEMA).await?;
        Ok(Self { db })
    }

    // ── Registration ──────────────────────────────────────────────────────────

    /// Register a service capability. If the entry already exists, it is updated.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] on database failure.
    #[instrument(name = "registry.register", skip(self, entry))]
    pub async fn register(&self, entry: ServiceEntry) -> Result<(), RegistryError> {
        let existing = entity::Entity::find_by_id(&entry.id).one(&self.db).await?;

        if existing.is_some() {
            let mut active = entity::ActiveModel {
                id: Set(entry.id.clone()),
                service_id: Set(entry.service_id.clone()),
                capability: Set(entry.capability.clone()),
                endpoint: Set(entry.endpoint.clone()),
                status: Set(entry.status.to_string().to_lowercase()),
                registered_at: Set(entry.registered_at.to_rfc3339()),
            };
            active.endpoint = Set(entry.endpoint);
            active.status = Set(entry.status.to_string().to_lowercase());
            active.update(&self.db).await?;
        } else {
            entity::ActiveModel {
                id: Set(entry.id),
                service_id: Set(entry.service_id),
                capability: Set(entry.capability),
                endpoint: Set(entry.endpoint),
                status: Set(entry.status.to_string().to_lowercase()),
                registered_at: Set(entry.registered_at.to_rfc3339()),
            }
            .insert(&self.db)
            .await?;
        }
        Ok(())
    }

    /// Deregister all capabilities of a service (called on shutdown).
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] on database failure.
    #[instrument(name = "registry.deregister")]
    pub async fn deregister(&self, service_id: &str) -> Result<(), RegistryError> {
        let entries = entity::Entity::find()
            .filter(entity::Column::ServiceId.eq(service_id))
            .all(&self.db)
            .await?;

        for model in entries {
            let active: entity::ActiveModel = model.into();
            active.delete(&self.db).await?;
        }
        Ok(())
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// All services registered for a specific capability.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] on database or deserialization failure.
    pub async fn by_capability(
        &self,
        capability: &str,
    ) -> Result<Vec<ServiceEntry>, RegistryError> {
        entity::Entity::find()
            .filter(entity::Column::Capability.eq(capability))
            .all(&self.db)
            .await?
            .into_iter()
            .map(ServiceEntry::try_from)
            .collect()
    }

    /// All capabilities registered by a specific service.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] on database or deserialization failure.
    pub async fn by_service(&self, service_id: &str) -> Result<Vec<ServiceEntry>, RegistryError> {
        entity::Entity::find()
            .filter(entity::Column::ServiceId.eq(service_id))
            .all(&self.db)
            .await?
            .into_iter()
            .map(ServiceEntry::try_from)
            .collect()
    }

    /// All registered entries.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] on database or deserialization failure.
    pub async fn all(&self) -> Result<Vec<ServiceEntry>, RegistryError> {
        entity::Entity::find()
            .all(&self.db)
            .await?
            .into_iter()
            .map(ServiceEntry::try_from)
            .collect()
    }

    /// Find the endpoint of the first `Up` service for a capability.
    ///
    /// Returns `None` if no service is registered or all are down.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] on database failure.
    pub async fn endpoint_for_capability(
        &self,
        capability: &str,
    ) -> Result<Option<String>, RegistryError> {
        let entries = self.by_capability(capability).await?;
        Ok(entries
            .into_iter()
            .find(|e| e.status == ServiceStatus::Up)
            .map(|e| e.endpoint))
    }

    /// Update the status of a specific entry.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError::NotFound`] if the id does not exist, or a database error.
    pub async fn set_status(&self, id: &str, status: ServiceStatus) -> Result<(), RegistryError> {
        let model = entity::Entity::find_by_id(id)
            .one(&self.db)
            .await?
            .ok_or_else(|| RegistryError::NotFound { id: id.to_owned() })?;
        let mut active: entity::ActiveModel = model.into();
        active.status = Set(status.to_string().to_lowercase());
        active.update(&self.db).await?;
        Ok(())
    }
}
