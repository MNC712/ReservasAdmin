//! Modelos de configuración: datos del hotel, colores por estado,
//! parámetros de backup y mapeo de columnas Excel.

use serde::{Deserialize, Serialize};

/// Datos generales del hotel (fila única id = 1).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct HotelConfig {
    pub id: i64,
    pub nombre: String,
    pub direccion: String,
    pub telefono: String,
    pub logo_path: String,
    pub moneda: String,
}

/// Color hexadecimal asociado a un estado de reserva (para el calendario).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ColorEstado {
    pub estado: String,
    pub color_hex: String,
}

impl ColorEstado {
    /// Convierte "#RRGGBB" a egui::Color32 (fallback: gris).
    pub fn a_color32(&self) -> egui::Color32 {
        let hex = self.color_hex.trim_start_matches('#');
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(128);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(128);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(128);
            egui::Color32::from_rgb(r, g, b)
        } else {
            egui::Color32::GRAY
        }
    }

    /// Asigna el color desde un egui::Color32.
    pub fn desde_color32(&mut self, c: egui::Color32) {
        self.color_hex = format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b());
    }
}

/// Configuración de copias de seguridad (fila única id = 1).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ConfigBackup {
    pub id: i64,
    pub carpeta: String,
    pub intervalo_min: i64,
    pub retencion_dias: i64,
}

/// Mapeo campo interno -> columna de Excel (letra "A".."Z" o nombre de cabecera).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExcelColumnMap {
    pub campo: String,
    pub columna_excel: String,
}
