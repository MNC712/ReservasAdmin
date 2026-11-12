//! Modelos de datos del dominio hotelero.

pub mod cliente;
pub mod configuracion;
pub mod habitacion;
pub mod huesped;
pub mod movimiento;
pub mod reserva;

pub use cliente::Cliente;
pub use configuracion::{ColorEstado, ConfigBackup, ExcelColumnMap, HotelConfig};
pub use habitacion::Habitacion;
pub use huesped::Huesped;
pub use movimiento::{Movimiento, TipoMovimiento};
pub use reserva::{EstadoReserva, Reserva, ReservaDetalle, ReservaInput};
