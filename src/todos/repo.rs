//! Todo storage in PostgreSQL.

use axum::extract::FromRef;
use sqlx::PgPool;

use super::model::{Change, ListParams, NewTodo, Title, Todo, TodoPatch};

/// The todos table. Cheap to clone: it only holds a pool handle.
#[derive(Debug, Clone)]
pub(crate) struct Todos {
    pool: PgPool,
}

impl FromRef<PgPool> for Todos {
    fn from_ref(pool: &PgPool) -> Self {
        Self { pool: pool.clone() }
    }
}

impl Todos {
    /// Lists todos, newest first, optionally filtered by completion state.
    pub(crate) async fn list(&self, params: ListParams) -> sqlx::Result<Vec<Todo>> {
        sqlx::query_as(
            "SELECT id, title, description, completed, created_at, updated_at
             FROM todos
             WHERE $1::boolean IS NULL OR completed = $1
             ORDER BY created_at DESC, id DESC",
        )
        .bind(params.completed)
        .fetch_all(&self.pool)
        .await
    }

    /// Finds one todo by id.
    pub(crate) async fn find(&self, id: i64) -> sqlx::Result<Option<Todo>> {
        sqlx::query_as(
            "SELECT id, title, description, completed, created_at, updated_at
             FROM todos
             WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
    }

    /// Inserts a new, not-yet-completed todo.
    pub(crate) async fn create(&self, new: NewTodo) -> sqlx::Result<Todo> {
        sqlx::query_as(
            "INSERT INTO todos (title, description)
             VALUES ($1, $2)
             RETURNING id, title, description, completed, created_at, updated_at",
        )
        .bind(new.title.as_str())
        .bind(new.description)
        .fetch_one(&self.pool)
        .await
    }

    /// Applies a partial update in one statement. Returns `None` if no todo has `id`.
    pub(crate) async fn update(&self, id: i64, patch: TodoPatch) -> sqlx::Result<Option<Todo>> {
        let (replace_description, description) = match patch.description {
            Change::Keep => (false, None),
            Change::Clear => (true, None),
            Change::Set(description) => (true, Some(description)),
        };
        sqlx::query_as(
            "UPDATE todos
             SET title       = COALESCE($2, title),
                 description = CASE WHEN $3 THEN $4 ELSE description END,
                 completed   = COALESCE($5, completed),
                 updated_at  = now()
             WHERE id = $1
             RETURNING id, title, description, completed, created_at, updated_at",
        )
        .bind(id)
        .bind(patch.title.as_ref().map(Title::as_str))
        .bind(replace_description)
        .bind(description)
        .bind(patch.completed)
        .fetch_optional(&self.pool)
        .await
    }

    /// Deletes a todo. Returns whether one existed.
    pub(crate) async fn delete(&self, id: i64) -> sqlx::Result<bool> {
        let result = sqlx::query("DELETE FROM todos WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }
}
