//! The OpenAPI document and the router it is generated from.

use sqlx::PgPool;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::{health, todos};

/// Top-level OpenAPI metadata. Paths and schemas are collected from the routes.
#[derive(OpenApi)]
#[openapi(
    info(title = "Todo API", description = "A REST API for managing todos."),
    tags(
        (name = "todos", description = "Create, read, update, and delete todos"),
        (name = "health", description = "Service health"),
    ),
)]
struct ApiDoc;

/// All API routes, each registered together with its OpenAPI operation.
pub(crate) fn router() -> OpenApiRouter<PgPool> {
    OpenApiRouter::with_openapi(ApiDoc::openapi())
        .routes(routes!(health::health))
        .merge(todos::routes())
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::health("get", "/health")]
    #[case::list("get", "/todos")]
    #[case::create("post", "/todos")]
    #[case::find("get", "/todos/{id}")]
    #[case::update("patch", "/todos/{id}")]
    #[case::delete("delete", "/todos/{id}")]
    fn spec_documents_operation(#[case] method: &str, #[case] path: &str) {
        let (_, api) = router().split_for_parts();

        let item = api
            .paths
            .paths
            .get(path)
            .unwrap_or_else(|| panic!("{path} is missing from the spec"));
        let operation = match method {
            "get" => item.get.as_ref(),
            "post" => item.post.as_ref(),
            "patch" => item.patch.as_ref(),
            "delete" => item.delete.as_ref(),
            other => panic!("unexpected method {other}"),
        };
        assert!(operation.is_some(), "{method} {path} is missing");
    }

    #[rstest]
    #[case("Todo")]
    #[case("NewTodo")]
    #[case("TodoPatch")]
    #[case("ErrorBody")]
    #[case("Health")]
    fn spec_has_schema(#[case] name: &str) {
        let (_, api) = router().split_for_parts();

        let components = api.components.expect("spec should have components");
        assert!(
            components.schemas.contains_key(name),
            "schema {name} is missing"
        );
    }
}
