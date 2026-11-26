//! The API error type and its JSON representation.

use axum::Json;
use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use utoipa::ToSchema;

/// Everything a handler can fail with. Each variant maps to one HTTP status.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
    /// The requested todo does not exist.
    #[error("todo not found")]
    NotFound,
    /// The request could not be parsed or failed validation.
    #[error("{message}")]
    InvalidRequest { status: StatusCode, message: String },
    /// The database failed. Details are logged, never sent to the client.
    #[error("internal server error")]
    Database(#[from] sqlx::Error),
}

/// The JSON body of every error response.
#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct ErrorBody {
    /// A human-readable description of what went wrong.
    #[schema(example = "todo not found")]
    pub(crate) error: String,
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::InvalidRequest { status, .. } => *status,
            Self::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        if let Self::Database(err) = &self {
            tracing::error!(error = %err, "database error");
        }
        let body = ErrorBody {
            error: self.to_string(),
        };
        (self.status(), Json(body)).into_response()
    }
}

macro_rules! from_rejection {
    ($($rejection:ty),+) => {$(
        impl From<$rejection> for ApiError {
            fn from(rejection: $rejection) -> Self {
                Self::InvalidRequest {
                    status: rejection.status(),
                    message: rejection.body_text(),
                }
            }
        }
    )+};
}

from_rejection!(JsonRejection, PathRejection, QueryRejection);

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;
    use rstest::rstest;
    use serde_json::{Value, json};

    use super::*;

    #[rstest]
    #[case::not_found(ApiError::NotFound, StatusCode::NOT_FOUND, "todo not found")]
    #[case::invalid_request(
        ApiError::InvalidRequest { status: StatusCode::UNPROCESSABLE_ENTITY, message: "bad title".to_owned() },
        StatusCode::UNPROCESSABLE_ENTITY,
        "bad title",
    )]
    #[case::database_hides_details(
        ApiError::Database(sqlx::Error::Protocol("secret connection detail".to_owned())),
        StatusCode::INTERNAL_SERVER_ERROR,
        "internal server error",
    )]
    #[tokio::test]
    async fn into_response(
        #[case] error: ApiError,
        #[case] expected_status: StatusCode,
        #[case] expected_message: &str,
    ) {
        let response = error.into_response();

        assert_eq!(response.status(), expected_status);
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body, json!({ "error": expected_message }));
    }
}
