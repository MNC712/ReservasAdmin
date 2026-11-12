//! Servicio de copias de seguridad: job periódico y ejecución manual.
//! Cada backup crea `<carpeta>/backup_YYYYMMDD_HHMMSS/` con una copia del
//! archivo .db y un `reservas.xlsx` exportado. Limpia backups antiguos
//! según la retención configurada.

use chrono::{Duration, Local, NaiveDateTime};
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::AppResult;
use crate::models::ConfigBackup;
use crate::repos;
use crate::state::AppState;

/// Lee la configuración efectiva de backup (BD tiene prioridad sobre .env).
async fn config_efectiva(state: &AppState) -> ConfigBackup {
    repos::config::obtener_backup(&state.db)
        .await
        .unwrap_or_else(|_| ConfigBackup {
            id: 1,
            carpeta: state.config.backup_dir.clone(),
            intervalo_min: state.config.backup_interval_min as i64,
            retencion_dias: state.config.backup_retention_dias,
        })
}

/// Ejecuta un backup ahora mismo. Devuelve la carpeta creada.
pub async fn ejecutar_backup_ahora(state: &Arc<AppState>) -> AppResult<PathBuf> {
    let cfg = config_efectiva(state).await;
    let marca = Local::now().format("%Y%m%d_%H%M%S");
    let carpeta = PathBuf::from(&cfg.carpeta).join(format!("backup_{}", marca));
    tokio::fs::create_dir_all(&carpeta).await?;

    // 1) Copia del archivo .db (si no es en memoria)
    if let Some(ruta_db) = state.config.ruta_archivo_db() {
        if ruta_db.exists() {
            let destino = carpeta.join("hotel.db");
            tokio::fs::copy(&ruta_db, &destino).await?;
        }
    }

    // 2) Export de reservas a Excel dentro de la carpeta del backup
    let ruta_excel = carpeta.join("reservas.xlsx");
    crate::excel::export::exportar_reservas(&state.db, ruta_excel.to_string_lossy().as_ref())
        .await?;

    // 3) Limpieza de backups antiguos
    limpiar_backups_antiguos(&cfg).await?;

    tracing::info!("backup completado en {}", carpeta.display());
    Ok(carpeta)
}

/// Borra carpetas `backup_YYYYMMDD_HHMMSS` más antiguas que `retencion_dias`.
async fn limpiar_backups_antiguos(cfg: &ConfigBackup) -> AppResult<()> {
    let limite = Local::now().naive_local() - Duration::days(cfg.retencion_dias);
    let raiz = PathBuf::from(&cfg.carpeta);
    if !raiz.exists() {
        return Ok(());
    }
    let mut entradas = tokio::fs::read_dir(&raiz).await?;
    while let Some(entrada) = entradas.next_entry().await? {
        let nombre = entrada.file_name().to_string_lossy().to_string();
        let Some(fecha_txt) = nombre.strip_prefix("backup_") else {
            continue;
        };
        if let Ok(fecha) = NaiveDateTime::parse_from_str(fecha_txt, "%Y%m%d_%H%M%S") {
            if fecha < limite && entrada.file_type().await.map(|t| t.is_dir()).unwrap_or(false)
            {
                tracing::info!("eliminando backup antiguo: {}", nombre);
                tokio::fs::remove_dir_all(entrada.path()).await?;
            }
        }
    }
    Ok(())
}

/// Job periódico de backup: espera `intervalo_min` entre ejecuciones.
/// Se relanza en bucle; los errores se registran y no detienen el job.
pub async fn job_backup(state: Arc<AppState>) {
    loop {
        let intervalo_min = config_efectiva(&state).await.intervalo_min.max(1) as u64;
        tokio::time::sleep(tokio::time::Duration::from_secs(intervalo_min * 60)).await;
        if let Err(e) = ejecutar_backup_ahora(&state).await {
            tracing::error!("error en job de backup: {}", e);
        }
    }
}
