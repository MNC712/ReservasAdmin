//! Tipo de error unificado de la aplicación.
//! Implementa IntoResponse de axum para devolver códigos HTTP adecuados.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Errores de negocio e infraestructura del sistema.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("error de base de datos: {0}")]
    Db(#[from] sqlx::Error),

    #[error("error de migración: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error("solapamiento de reserva: {0}")]
    Solapamiento(String),

    #[error("no encontrado: {0}")]
    NoEncontrado(String),

    #[error("validación: {0}")]
    Validacion(String),

    #[error("excel: {0}")]
    Excel(String),

    #[error("pdf: {0}")]
    Pdf(String),

    #[error("hostelmate: {0}")]
    HostelMate(String),

    #[error("error de E/S: {0}")]
    Io(#[from] std::io::Error),
}

/// Alias de resultado usado en servicios, repos y handlers.
pub type AppResult<T> = Result<T, AppError>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // 409 para solapamientos, 404 para no encontrado, 400 validación, 500 el resto
        let codigo = match &self {
            AppError::Solapamiento(_) => StatusCode::CONFLICT,
            AppError::NoEncontrado(_) => StatusCode::NOT_FOUND,
            AppError::Validacion(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        let cuerpo = Json(json!({ "error": self.to_string() }));
        (codigo, cuerpo).into_response()
    }
}
