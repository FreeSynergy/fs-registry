//! Error type for registry operations.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("service not found: {id}")]
    NotFound { id: String },

    #[error("serialisation error: {0}")]
    Json(#[from] serde_json::Error),
}
