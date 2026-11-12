//! Estado compartido de la aplicación, clonado en `Arc` para el servidor axum,
//! los jobs en background (backup, polling HostelMate) y la UI egui.

use sqlx::SqlitePool;
use std::sync::Arc;

use crate::config::AppConfig;
use crate::error::AppResult;
use crate::hostelmate::client::HostelMateHttpClient;

/// Estado global compartido.
pub struct AppState {
    pub db: SqlitePool,
    pub config: AppConfig,
    pub hostelmate: HostelMateHttpClient,
}

impl AppState {
    /// Construye el estado a partir de la configuración: crea pool y migra.
    pub async fn inicializar(config: AppConfig) -> AppResult<Arc<Self>> {
        let db = crate::db::inicializar(&config.database_url).await?;
        let hostelmate = HostelMateHttpClient::new(
            config.hostelmate_base_url.clone(),
            config.hostelmate_api_key.clone(),
        );
        Ok(Arc::new(Self {
            db,
            config,
            hostelmate,
        }))
    }

    /// Estado para tests: pool en memoria ya migrado y HostelMate deshabilitado.
    pub async fn for_tests(db: SqlitePool) -> Arc<Self> {
        let config = AppConfig::for_tests();
        let hostelmate = HostelMateHttpClient::new(
            config.hostelmate_base_url.clone(),
            String::new(), // sin API key -> cliente deshabilitado
        );
        Arc::new(Self {
            db,
            config,
            hostelmate,
        })
    }
}
