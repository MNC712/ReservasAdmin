//! Integración con HostelMate (channel manager).
//! Contiene el cliente HTTP (endpoints PLACEHOLDER con TODOs) y el job
//! de polling periódico que hace upsert de reservas externas por sync_id.

pub mod client;

pub use client::{HostelMateApi, HostelMateHttpClient, ReservaExterna};

use std::sync::Arc;

use chrono::Local;

use crate::error::AppResult;
use crate::state::AppState;

/// Intervalo del polling de HostelMate (minutos).
const POLLING_INTERVALO_MIN: u64 = 5;

/// Upsert de una reserva externa (webhook o polling) identificada por `sync_id`.
///
/// TODO(HostelMate): ajustar el mapeo de campos según la documentación oficial
/// (identificación del titular, habitaciones, tarifa, estado, etc.).
pub async fn upsert_reserva_externa(
    state: &Arc<AppState>,
    ext: &ReservaExterna,
) -> AppResult<()> {
    let db = &state.db;
    let ahora = Local::now().naive_local();

    // TODO(HostelMate): mapear el titular real. De momento se crea/reutiliza
    // un cliente genérico "HostelMate" para no perder la reserva.
    let cliente_id: i64 = match sqlx::query_scalar::<_, i64>(
        "SELECT id FROM clientes WHERE documento = 'HOSTELMATE' LIMIT 1",
    )
    .fetch_optional(db)
    .await?
    {
        Some(id) => id,
        None => {
            sqlx::query(
                "INSERT INTO clientes (nombre, apellidos, documento, email, telefono)
                 VALUES (?1, '', 'HOSTELMATE', '', '')",
            )
            .bind(&ext.titular_nombre)
            .execute(db)
            .await?
            .last_insert_rowid()
        }
    };

    // Upsert por sync_id
    let existente: Option<i64> =
        sqlx::query_scalar("SELECT id FROM reservas WHERE sync_id = ?1")
            .bind(&ext.sync_id)
            .fetch_optional(db)
            .await?;

    match existente {
        Some(id) => {
            sqlx::query(
                "UPDATE reservas SET fecha_inicio = ?2, fecha_fin = ?3, estado = ?4,
                 tarifa_total = ?5, notas = ?6, actualizado_en = ?7 WHERE id = ?1",
            )
            .bind(id)
            .bind(ext.fecha_inicio)
            .bind(ext.fecha_fin)
            .bind(&ext.estado)
            .bind(ext.tarifa_total)
            .bind(&ext.notas)
            .bind(ahora)
            .execute(db)
            .await?;
        }
        None => {
            sqlx::query(
                "INSERT INTO reservas (sync_id, cliente_id, fecha_inicio, fecha_fin, estado, tarifa_total, notas, origen, creado_en, actualizado_en)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'hostelmate', ?8, ?8)",
            )
            .bind(&ext.sync_id)
            .bind(cliente_id)
            .bind(ext.fecha_inicio)
            .bind(ext.fecha_fin)
            .bind(&ext.estado)
            .bind(ext.tarifa_total)
            .bind(&ext.notas)
            .bind(ahora)
            .execute(db)
            .await?;
            // TODO(HostelMate): asociar habitaciones/huéspedes cuando se conozca
            // el formato real del payload (mapeo numero_habitacion -> habitacion_id).
        }
    }
    Ok(())
}

/// Job de polling periódico: consulta reservas actualizadas en HostelMate
/// y hace upsert en SQLite + reexport del Excel. Errores solo se registran.
pub async fn job_polling(state: Arc<AppState>) {
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(POLLING_INTERVALO_MIN * 60)).await;

        if !state.hostelmate.habilitado() {
            continue; // sin API key configurada: nada que hacer
        }

        // TODO(HostelMate): persistir/recuperar la marca de última sincronización
        // (p.ej. en una tabla de metadatos) en lugar de usar un timestamp fijo.
        let desde = Local::now().naive_local() - chrono::Duration::days(1);

        match state.hostelmate.listar_reservas_desde(desde).await {
            Ok(reservas) => {
                for ext in &reservas {
                    if let Err(e) = upsert_reserva_externa(&state, ext).await {
                        tracing::error!("error en upsert de reserva HostelMate: {}", e);
                    }
                }
                if !reservas.is_empty() {
                    if let Err(e) = crate::excel::export::exportar_reservas(
                        &state.db,
                        &state.config.excel_backup_path,
                    )
                    .await
                    {
                        tracing::error!("error reexportando Excel tras polling: {}", e);
                    }
                }
            }
            Err(e) => tracing::error!("error en polling de HostelMate: {}", e),
        }
    }
}
