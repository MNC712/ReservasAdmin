//! Modelo Reserva: cabecera + detalle (habitaciones y huéspedes asociados).
//! El estado se persiste como TEXT y se modela con el enum `EstadoReserva`.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteTypeInfo, SqliteValueRef};
use sqlx::{Decode, Sqlite, Type};
use std::fmt;
use std::str::FromStr;

use super::{Cliente, Habitacion, Huesped};

/// Estados posibles de una reserva.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EstadoReserva {
    Pendiente,
    Confirmada,
    CheckIn,
    Cancelada,
}

impl EstadoReserva {
    /// Todos los estados, para combos de la UI.
    pub const TODOS: [EstadoReserva; 4] = [
        EstadoReserva::Pendiente,
        EstadoReserva::Confirmada,
        EstadoReserva::CheckIn,
        EstadoReserva::Cancelada,
    ];

    /// Etiqueta en español para la UI.
    pub fn etiqueta(&self) -> &'static str {
        match self {
            EstadoReserva::Pendiente => "Pendiente",
            EstadoReserva::Confirmada => "Confirmada",
            EstadoReserva::CheckIn => "Check-in",
            EstadoReserva::Cancelada => "Cancelada",
        }
    }
}

impl fmt::Display for EstadoReserva {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Valor persistido en BD (minúsculas, tal como en el CHECK de la tabla)
        let s = match self {
            EstadoReserva::Pendiente => "pendiente",
            EstadoReserva::Confirmada => "confirmada",
            EstadoReserva::CheckIn => "checkin",
            EstadoReserva::Cancelada => "cancelada",
        };
        write!(f, "{}", s)
    }
}

impl FromStr for EstadoReserva {
    type Err = crate::error::AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pendiente" => Ok(EstadoReserva::Pendiente),
            "confirmada" => Ok(EstadoReserva::Confirmada),
            "checkin" => Ok(EstadoReserva::CheckIn),
            "cancelada" => Ok(EstadoReserva::Cancelada),
            otro => Err(crate::error::AppError::Validacion(format!(
                "estado de reserva desconocido: '{}'",
                otro
            ))),
        }
    }
}

// Persistencia del enum como TEXT en SQLite (sin macros de sqlx)
impl Type<Sqlite> for EstadoReserva {
    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'r> Decode<'r, Sqlite> for EstadoReserva {
    fn decode(value: SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as Decode<Sqlite>>::decode(value)?;
        Ok(s.parse()?)
    }
}

/// Reserva tal y como se almacena en la tabla `reservas`.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Reserva {
    pub id: i64,
    /// UUID v4 usado como clave de idempotencia/sincronización con HostelMate
    pub sync_id: String,
    pub cliente_id: i64,
    /// Fecha de check-in, ISO 'YYYY-MM-DD'
    pub fecha_inicio: NaiveDate,
    /// Fecha de check-out (exclusiva), ISO 'YYYY-MM-DD'
    pub fecha_fin: NaiveDate,
    pub estado: EstadoReserva,
    pub tarifa_total: f64,
    pub notas: String,
    /// Origen de la reserva: 'local', 'hostelmate', 'excel', ...
    pub origen: String,
    pub creado_en: chrono::NaiveDateTime,
    pub actualizado_en: chrono::NaiveDateTime,
}

/// Reserva "enriquecida" para mostrar en UI/API: incluye titular,
/// habitaciones y huéspedes asociados.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservaDetalle {
    #[serde(flatten)]
    pub reserva: Reserva,
    pub titular: Cliente,
    pub habitaciones: Vec<Habitacion>,
    pub huespedes: Vec<Huesped>,
}

impl ReservaDetalle {
    /// Números de habitación separados por coma, para tablas.
    pub fn habitaciones_resumen(&self) -> String {
        self.habitaciones
            .iter()
            .map(|h| h.numero.clone())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Datos necesarios para crear/actualizar una reserva (input de la UI/API).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservaInput {
    pub cliente_id: i64,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin: NaiveDate,
    pub estado: EstadoReserva,
    pub tarifa_total: f64,
    #[serde(default)]
    pub notas: String,
    pub habitacion_ids: Vec<i64>,
    #[serde(default)]
    pub huesped_ids: Vec<i64>,
}
