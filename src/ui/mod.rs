//! Interfaz de escritorio con egui/eframe.
//! La UI corre en el hilo principal y accede a los servicios async mediante
//! `tokio::runtime::Handle::block_on` (el hilo de UI no es worker de tokio).

pub mod app;
pub mod calendario;
pub mod configuracion;
pub mod dashboard;
pub mod lista_reservas;
pub mod movimientos;
pub mod nueva_reserva;
pub mod tema;

pub use app::HotelApp;
