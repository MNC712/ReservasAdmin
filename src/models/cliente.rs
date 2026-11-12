//! Modelo Cliente: titular de la reserva (quien paga / firma).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Cliente {
    pub id: i64,
    pub nombre: String,
    pub apellidos: String,
    pub documento: String,
    pub email: String,
    pub telefono: String,
}

impl Cliente {
    /// Nombre completo para mostrar en listas y combos.
    pub fn nombre_completo(&self) -> String {
        if self.apellidos.is_empty() {
            self.nombre.clone()
        } else {
            format!("{} {}", self.nombre, self.apellidos)
        }
    }
}
