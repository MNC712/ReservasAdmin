//! Repositorio de configuración: hotel, colores por estado, backup y mapa Excel.

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::{ColorEstado, ConfigBackup, ExcelColumnMap, HotelConfig};

/// Obtiene la configuración del hotel (fila única id = 1).
pub async fn obtener_hotel(db: &SqlitePool) -> AppResult<HotelConfig> {
    sqlx::query_as::<_, HotelConfig>(
        "SELECT id, nombre, direccion, telefono, logo_path, moneda FROM hotel_config WHERE id = 1",
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("configuración del hotel".to_string()))
}

/// Actualiza la configuración del hotel.
pub async fn actualizar_hotel(db: &SqlitePool, cfg: &HotelConfig) -> AppResult<()> {
    sqlx::query(
        "UPDATE hotel_config SET nombre = ?1, direccion = ?2, telefono = ?3, logo_path = ?4, moneda = ?5 WHERE id = 1",
    )
    .bind(&cfg.nombre)
    .bind(&cfg.direccion)
    .bind(&cfg.telefono)
    .bind(&cfg.logo_path)
    .bind(&cfg.moneda)
    .execute(db)
    .await?;
    Ok(())
}

/// Lista los colores configurados por estado.
pub async fn listar_colores(db: &SqlitePool) -> AppResult<Vec<ColorEstado>> {
    Ok(
        sqlx::query_as::<_, ColorEstado>("SELECT estado, color_hex FROM config_colores ORDER BY estado")
            .fetch_all(db)
            .await?,
    )
}

/// Actualiza (o inserta) el color de un estado.
pub async fn actualizar_color(db: &SqlitePool, estado: &str, color_hex: &str) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO config_colores (estado, color_hex) VALUES (?1, ?2)
         ON CONFLICT(estado) DO UPDATE SET color_hex = excluded.color_hex",
    )
    .bind(estado)
    .bind(color_hex)
    .execute(db)
    .await?;
    Ok(())
}

/// Obtiene la configuración de backup (fila única id = 1).
pub async fn obtener_backup(db: &SqlitePool) -> AppResult<ConfigBackup> {
    sqlx::query_as::<_, ConfigBackup>(
        "SELECT id, carpeta, intervalo_min, retencion_dias FROM config_backup WHERE id = 1",
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("configuración de backup".to_string()))
}

/// Actualiza la configuración de backup.
pub async fn actualizar_backup(db: &SqlitePool, cfg: &ConfigBackup) -> AppResult<()> {
    sqlx::query(
        "UPDATE config_backup SET carpeta = ?1, intervalo_min = ?2, retencion_dias = ?3 WHERE id = 1",
    )
    .bind(&cfg.carpeta)
    .bind(cfg.intervalo_min)
    .bind(cfg.retencion_dias)
    .execute(db)
    .await?;
    Ok(())
}

/// Lista el mapeo campo -> columna de Excel.
pub async fn listar_excel_map(db: &SqlitePool) -> AppResult<Vec<ExcelColumnMap>> {
    Ok(sqlx::query_as::<_, ExcelColumnMap>(
        "SELECT campo, columna_excel FROM excel_column_map ORDER BY campo",
    )
    .fetch_all(db)
    .await?)
}

/// Actualiza el mapeo de un campo.
pub async fn actualizar_excel_map(
    db: &SqlitePool,
    campo: &str,
    columna: &str,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO excel_column_map (campo, columna_excel) VALUES (?1, ?2)
         ON CONFLICT(campo) DO UPDATE SET columna_excel = excluded.columna_excel",
    )
    .bind(campo)
    .bind(columna)
    .execute(db)
    .await?;
    Ok(())
}
