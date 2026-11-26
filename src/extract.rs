//! Extractors that wrap axum's own but reject with [`ApiError`], so every error the API
//! returns has the same JSON shape.

use axum::extract::{FromRequest, FromRequestParts};

use crate::error::ApiError;

/// [`axum::Json`] that rejects with [`ApiError`].
#[derive(Debug, FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub(crate) struct Json<T>(pub(crate) T);

/// [`axum::extract::Path`] that rejects with [`ApiError`].
#[derive(Debug, FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(ApiError))]
pub(crate) struct Path<T>(pub(crate) T);

/// [`axum::extract::Query`] that rejects with [`ApiError`].
#[derive(Debug, FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(ApiError))]
pub(crate) struct Query<T>(pub(crate) T);
