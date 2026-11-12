//! Carga de configuración desde variables de entorno / archivo .env.
//! Usa dotenvy; si falta alguna variable se aplican valores por defecto sensatos.

use std::env;

/// Configuración global de la aplicación (leída de .env / entorno).
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// URL de conexión SQLite, p.ej. `sqlite:hotel.db`
    pub database_url: String,
    /// Dirección de escucha del servidor HTTP axum
    pub server_addr: String,
    /// URL base de la API de HostelMate (PLACEHOLDER)
    pub hostelmate_base_url: String,
    /// API key de HostelMate
    pub hostelmate_api_key: String,
    /// Ruta del Excel de respaldo que se reexporta tras cada mutación
    pub excel_backup_path: String,
    /// Carpeta donde se generan los comprobantes PDF
    pub pdf_output_dir: String,
    /// Carpeta raíz de copias de seguridad
    pub backup_dir: String,
    /// Intervalo del job de backup (minutos)
    pub backup_interval_min: u64,
    /// Retención de backups (días)
    pub backup_retention_dias: i64,
}

impl AppConfig {
    /// Lee la configuración del entorno con defaults razonables.
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:hotel.db".to_string()),
            server_addr: env::var("SERVER_ADDR")
                .unwrap_or_else(|_| "127.0.0.1:3000".to_string()),
            hostelmate_base_url: env::var("HOSTELMATE_BASE_URL")
                .unwrap_or_else(|_| "https://api.hostelmate.example.com".to_string()),
            hostelmate_api_key: env::var("HOSTELMATE_API_KEY").unwrap_or_default(),
            excel_backup_path: env::var("EXCEL_BACKUP_PATH")
                .unwrap_or_else(|_| "reservas_backup.xlsx".to_string()),
            pdf_output_dir: env::var("PDF_OUTPUT_DIR")
                .unwrap_or_else(|_| "comprobantes".to_string()),
            backup_dir: env::var("BACKUP_DIR").unwrap_or_else(|_| "backups".to_string()),
            backup_interval_min: env::var("BACKUP_INTERVAL_MIN")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            backup_retention_dias: env::var("BACKUP_RETENTION_DIAS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
        }
    }

    /// Configuración pensada para tests: BBDD en memoria y HostelMate deshabilitado.
    pub fn for_tests() -> Self {
        Self {
            database_url: "sqlite::memory:".to_string(),
            hostelmate_api_key: String::new(),
            excel_backup_path: std::env::temp_dir()
                .join("hotel_reservas_test_backup.xlsx")
                .to_string_lossy()
                .to_string(),
            pdf_output_dir: std::env::temp_dir()
                .join("hotel_reservas_test_pdf")
                .to_string_lossy()
                .to_string(),
            backup_dir: std::env::temp_dir()
                .join("hotel_reservas_test_backups")
                .to_string_lossy()
                .to_string(),
            ..Self::from_env()
        }
    }

    /// Extrae la ruta del archivo .db a partir del DATABASE_URL (`sqlite:ruta`).
    /// Devuelve None si es una base en memoria.
    pub fn ruta_archivo_db(&self) -> Option<std::path::PathBuf> {
        let sin_prefijo = self
            .database_url
            .strip_prefix("sqlite:")
            .unwrap_or(&self.database_url);
        if sin_prefijo.contains(":memory:") {
            return None;
        }
        Some(std::path::PathBuf::from(sin_prefijo))
    }
}
