//! Modelo Movimiento: entrada de la auditoría de cambios.

use serde::{Deserialize, Serialize};

/// Tipo de movimiento de auditoría.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TipoMovimiento {
    Creacion,
    Modificacion,
    Cancelacion,
}

impl std::fmt::Display for TipoMovimiento {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TipoMovimiento::Creacion => "creacion",
            TipoMovimiento::Modificacion => "modificacion",
            TipoMovimiento::Cancelacion => "cancelacion",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Movimiento {
    pub id: i64,
    /// 'creacion' | 'modificacion' | 'cancelacion'
    pub tipo: String,
    /// Entidad afectada, p.ej. 'reserva'
    pub entidad: String,
    pub id_entidad: i64,
    /// JSON del estado anterior (vacío si no aplica)
    pub datos_antes: String,
    /// JSON del estado posterior
    pub datos_despues: String,
    pub usuario: String,
    pub timestamp: chrono::NaiveDateTime,
}
