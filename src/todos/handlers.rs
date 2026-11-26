//! HTTP handlers for `/todos`, each annotated with its OpenAPI operation.

use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header::LOCATION;
use axum::response::IntoResponse;
use sqlx::PgPool;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::model::{ListParams, NewTodo, Todo, TodoPatch};
use super::repo::Todos;
use crate::error::{ApiError, ErrorBody};
use crate::extract::{Json, Path, Query};

const TAG: &str = "todos";

/// The `/todos` routes and their OpenAPI operations.
pub(crate) fn routes() -> OpenApiRouter<PgPool> {
    OpenApiRouter::new()
        .routes(routes!(list, create))
        .routes(routes!(find, update, delete))
}

/// Lists todos, newest first.
#[utoipa::path(
    get,
    path = "/todos",
    tag = TAG,
    params(ListParams),
    responses(
        (status = OK, description = "Matching todos, newest first", body = [Todo]),
        (status = BAD_REQUEST, description = "Invalid query string", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, body = ErrorBody),
    ),
)]
async fn list(
    State(todos): State<Todos>,
    Query(params): Query<ListParams>,
) -> Result<axum::Json<Vec<Todo>>, ApiError> {
    Ok(axum::Json(todos.list(params).await?))
}

/// Creates a todo.
#[utoipa::path(
    post,
    path = "/todos",
    tag = TAG,
    request_body = NewTodo,
    responses(
        (status = CREATED, description = "The created todo", body = Todo,
            headers(("location" = String, description = "URL of the new todo"))),
        (status = BAD_REQUEST, description = "Malformed JSON", body = ErrorBody),
        (status = UNSUPPORTED_MEDIA_TYPE, description = "Body is not JSON", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Invalid fields", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, body = ErrorBody),
    ),
)]
async fn create(
    State(todos): State<Todos>,
    Json(new): Json<NewTodo>,
) -> Result<impl IntoResponse, ApiError> {
    let todo = todos.create(new).await?;
    let location = format!("/todos/{}", todo.id);
    Ok((
        StatusCode::CREATED,
        [(LOCATION, location)],
        axum::Json(todo),
    ))
}

/// Fetches one todo.
#[utoipa::path(
    get,
    path = "/todos/{id}",
    tag = TAG,
    params(("id" = i64, Path, description = "Todo id")),
    responses(
        (status = OK, description = "The todo", body = Todo),
        (status = BAD_REQUEST, description = "Id is not an integer", body = ErrorBody),
        (status = NOT_FOUND, description = "No todo has this id", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, body = ErrorBody),
    ),
)]
async fn find(
    State(todos): State<Todos>,
    Path(id): Path<i64>,
) -> Result<axum::Json<Todo>, ApiError> {
    let todo = todos.find(id).await?.ok_or(ApiError::NotFound)?;
    Ok(axum::Json(todo))
}

/// Partially updates a todo. Absent fields are left unchanged.
#[utoipa::path(
    patch,
    path = "/todos/{id}",
    tag = TAG,
    params(("id" = i64, Path, description = "Todo id")),
    request_body = TodoPatch,
    responses(
        (status = OK, description = "The updated todo", body = Todo),
        (status = BAD_REQUEST, description = "Malformed JSON or id", body = ErrorBody),
        (status = NOT_FOUND, description = "No todo has this id", body = ErrorBody),
        (status = UNSUPPORTED_MEDIA_TYPE, description = "Body is not JSON", body = ErrorBody),
        (status = UNPROCESSABLE_ENTITY, description = "Invalid fields", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, body = ErrorBody),
    ),
)]
async fn update(
    State(todos): State<Todos>,
    Path(id): Path<i64>,
    Json(patch): Json<TodoPatch>,
) -> Result<axum::Json<Todo>, ApiError> {
    let todo = todos.update(id, patch).await?.ok_or(ApiError::NotFound)?;
    Ok(axum::Json(todo))
}

/// Deletes a todo.
#[utoipa::path(
    delete,
    path = "/todos/{id}",
    tag = TAG,
    params(("id" = i64, Path, description = "Todo id")),
    responses(
        (status = NO_CONTENT, description = "The todo was deleted"),
        (status = BAD_REQUEST, description = "Id is not an integer", body = ErrorBody),
        (status = NOT_FOUND, description = "No todo has this id", body = ErrorBody),
        (status = INTERNAL_SERVER_ERROR, body = ErrorBody),
    ),
)]
async fn delete(State(todos): State<Todos>, Path(id): Path<i64>) -> Result<StatusCode, ApiError> {
    if todos.delete(id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound)
    }
}
