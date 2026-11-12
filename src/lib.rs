//! Crate biblioteca del sistema de gestión de reservas hoteleras.
//! Expone todos los módulos para que el binario (`main.rs`) y los tests
//! de integración puedan reutilizar servicios, repos y modelos.

pub mod api;
pub mod config;
pub mod db;
pub mod error;
pub mod excel;
pub mod hostelmate;
pub mod models;
pub mod pdf;
pub mod repos;
pub mod services;
pub mod state;
pub mod ui;
