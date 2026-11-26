//! The health-check endpoint.

use axum::Json;
use axum::extract::State;
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::error::{ApiError, ErrorBody};

/// The health-check response.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct Health {
    /// Always `"ok"` when the service and its database are reachable.
    #[schema(example = "ok")]
    status: &'static str,
}

/// Reports whether the service is up and can reach its database.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = OK, description = "Service and database are healthy", body = Health),
        (status = INTERNAL_SERVER_ERROR, description = "Database unreachable", body = ErrorBody),
    ),
)]
pub(crate) async fn health(State(pool): State<PgPool>) -> Result<Json<Health>, ApiError> {
    sqlx::query("SELECT 1").execute(&pool).await?;
    Ok(Json(Health { status: "ok" }))
}
