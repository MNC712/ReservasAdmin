//! Modelo Huésped: acompañante/ocupante de una reserva.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Huesped {
    pub id: i64,
    pub nombre: String,
    pub apellidos: String,
    pub documento: String,
    /// Fecha de nacimiento en formato ISO 'YYYY-MM-DD' (o cadena vacía)
    pub fecha_nacimiento: String,
    pub nacionalidad: String,
}

impl Huesped {
    pub fn nombre_completo(&self) -> String {
        if self.apellidos.is_empty() {
            self.nombre.clone()
        } else {
            format!("{} {}", self.nombre, self.apellidos)
        }
    }
}
