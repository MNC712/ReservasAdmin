//! API HTTP con axum: router, handlers y arranque del servidor.

pub mod handlers;
pub mod routes;

pub use routes::{router, servir};
