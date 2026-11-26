//! A REST API for managing todos, backed by PostgreSQL.
//!
//! [`connect`] opens a connection pool and applies the embedded migrations; [`app`] builds
//! the axum [`Router`] that serves the API, its OpenAPI spec at `/api-docs/openapi.json`,
//! and Swagger UI at `/swagger-ui`.

pub mod config;
mod error;
mod extract;
mod health;
mod openapi;
mod todos;

use std::time::Duration;

use anyhow::Context as _;
use axum::Router;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;
use utoipa_swagger_ui::SwaggerUi;

/// Path the OpenAPI document is served at.
const OPENAPI_PATH: &str = "/api-docs/openapi.json";

/// Connects to the database at `database_url` and applies pending migrations.
///
/// # Errors
///
/// Returns an error if the database is unreachable, the URL is invalid, or a migration
/// fails to apply.
pub async fn connect(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
        .context("failed to connect to the database")?;
    sqlx::migrate!()
        .run(&pool)
        .await
        .context("failed to run database migrations")?;
    Ok(pool)
}

/// Builds the application router: the API routes, the OpenAPI spec, and Swagger UI.
pub fn app(pool: PgPool) -> Router {
    let (router, api) = openapi::router().split_for_parts();
    router
        .merge(SwaggerUi::new("/swagger-ui").url(OPENAPI_PATH, api))
        .layer(TraceLayer::new_for_http())
        .with_state(pool)
}
