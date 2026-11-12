//! Modelo Habitación.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Habitacion {
    pub id: i64,
    pub numero: String,
    pub tipo: String,
    pub capacidad: i64,
    pub precio_base: f64,
    /// SQLite guarda 0/1; sqlx lo mapea a bool automáticamente
    pub activa: bool,
}
