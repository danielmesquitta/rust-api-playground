//! The todos resource: its model, storage, and HTTP handlers.

mod handlers;
mod model;
mod repo;

pub(crate) use handlers::routes;
