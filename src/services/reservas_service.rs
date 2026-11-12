//! Servicio de reservas: creación, actualización y cancelación.
//!
//! Flujo de cada mutación:
//! 1. Validación (fechas coherentes + solapamiento por habitación).
//! 2. Transacción SQLite (cabecera + tablas puente + auditoría).
//! 3. Reexport del Excel de respaldo (best-effort, solo log si falla).
//! 4. Envío a HostelMate (best-effort, NO bloquea ni hace rollback).

use chrono::{Local, NaiveDate};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::hostelmate::HostelMateApi;
use crate::models::movimiento::TipoMovimiento;
use crate::models::{EstadoReserva, Reserva, ReservaDetalle, ReservaInput};
use crate::repos;
use crate::services::auditoria;
use crate::state::AppState;

/// Devuelve true si dos rangos [inicio, fin) se solapan.
/// Regla: contigüos NO solapan (el check-out coincide con el check-in siguiente).
pub fn fechas_solapan(
    inicio1: NaiveDate,
    fin1: NaiveDate,
    inicio2: NaiveDate,
    fin2: NaiveDate,
) -> bool {
    inicio1 < fin2 && inicio2 < fin1
}

/// Valida datos básicos y solapamiento de cada habitación seleccionada.
/// `excluir_reserva_id` = 0 cuando se crea una reserva nueva.
async fn validar(
    state: &AppState,
    input: &ReservaInput,
    excluir_reserva_id: i64,
) -> AppResult<()> {
    if input.fecha_inicio >= input.fecha_fin {
        return Err(AppError::Validacion(
            "la fecha de check-in debe ser anterior a la de check-out".to_string(),
        ));
    }
    if input.habitacion_ids.is_empty() {
        return Err(AppError::Validacion(
            "debe seleccionar al menos una habitación".to_string(),
        ));
    }
    // Comprobación de solapamiento en BD por cada habitación
    for hab_id in &input.habitacion_ids {
        if let Some(conflicto) = repos::reservas::reserva_solapada(
            &state.db,
            *hab_id,
            excluir_reserva_id,
            input.fecha_inicio,
            input.fecha_fin,
        )
        .await?
        {
            let hab = repos::habitaciones::obtener(&state.db, *hab_id).await?;
            return Err(AppError::Solapamiento(format!(
                "la habitación {} ya está reservada en esas fechas (reserva #{})",
                hab.numero, conflicto
            )));
        }
    }
    Ok(())
}

/// Tareas post-mutación: reexport de Excel y sincronización con HostelMate.
/// Ambas son best-effort: un fallo solo se registra en el log.
async fn post_mutacion(state: &AppState, reserva_id: i64, accion: AccionSync) {
    if let Err(e) = crate::excel::export::exportar_reservas(
        &state.db,
        &state.config.excel_backup_path,
    )
    .await
    {
        tracing::error!("error reexportando Excel de respaldo: {}", e);
    }

    match repos::reservas::detalle(&state.db, reserva_id).await {
        Ok(det) => {
            let resultado = match accion {
                AccionSync::Crear => state.hostelmate.crear_reserva(&det).await,
                AccionSync::Actualizar => state.hostelmate.actualizar_reserva(&det).await,
                AccionSync::Cancelar => state.hostelmate.cancelar_reserva(&det.reserva.sync_id).await,
            };
            if let Err(e) = resultado {
                tracing::error!("error sincronizando con HostelMate: {}", e);
            }
        }
        Err(e) => tracing::error!("error obteniendo detalle para HostelMate: {}", e),
    }
}

/// Acción de sincronización a notificar a HostelMate.
enum AccionSync {
    Crear,
    Actualizar,
    Cancelar,
}

/// Crea una reserva nueva con sus habitaciones y huéspedes.
pub async fn crear_reserva(
    state: &Arc<AppState>,
    input: &ReservaInput,
    usuario: &str,
) -> AppResult<ReservaDetalle> {
    validar(state, input, 0).await?;

    let ahora = Local::now().naive_local();
    // UUID v4 como clave de idempotencia/sincronización con HostelMate
    let sync_id = Uuid::new_v4().to_string();

    let mut tx = state.db.begin().await?;

    // Insert de cabecera
    let reserva_id = sqlx::query(
        "INSERT INTO reservas (sync_id, cliente_id, fecha_inicio, fecha_fin, estado, tarifa_total, notas, origen, creado_en, actualizado_en)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'local', ?8, ?8)",
    )
    .bind(&sync_id)
    .bind(input.cliente_id)
    .bind(input.fecha_inicio)
    .bind(input.fecha_fin)
    .bind(input.estado.to_string())
    .bind(input.tarifa_total)
    .bind(&input.notas)
    .bind(ahora)
    .execute(&mut *tx)
    .await?
    .last_insert_rowid();

    // Tablas puente
    for hab_id in &input.habitacion_ids {
        sqlx::query(
            "INSERT INTO reserva_habitaciones (reserva_id, habitacion_id) VALUES (?1, ?2)",
        )
        .bind(reserva_id)
        .bind(hab_id)
        .execute(&mut *tx)
        .await?;
    }
    for huesped_id in &input.huesped_ids {
        sqlx::query("INSERT INTO reserva_huespedes (reserva_id, huesped_id) VALUES (?1, ?2)")
            .bind(reserva_id)
            .bind(huesped_id)
            .execute(&mut *tx)
            .await?;
    }

    // Auditoría dentro de la misma transacción
    let despues = (input, sync_id.clone());
    auditoria::registrar_movimiento(
        &mut *tx,
        TipoMovimiento::Creacion,
        "reserva",
        reserva_id,
        None::<&Reserva>,
        Some(&despues),
        usuario,
    )
    .await?;

    tx.commit().await?;

    post_mutacion(state, reserva_id, AccionSync::Crear).await;
    repos::reservas::detalle(&state.db, reserva_id).await
}

/// Actualiza una reserva existente (cabecera + asociaciones).
pub async fn actualizar_reserva(
    state: &Arc<AppState>,
    reserva_id: i64,
    input: &ReservaInput,
    usuario: &str,
) -> AppResult<ReservaDetalle> {
    let antes = repos::reservas::detalle(&state.db, reserva_id).await?;
    // No se permite reactivar/editar una cancelada salvo cambio explícito de estado:
    // la validación de solapamiento excluye la propia reserva.
    validar(state, input, reserva_id).await?;

    let ahora = Local::now().naive_local();
    let mut tx = state.db.begin().await?;

    sqlx::query(
        "UPDATE reservas SET cliente_id = ?2, fecha_inicio = ?3, fecha_fin = ?4, estado = ?5,
         tarifa_total = ?6, notas = ?7, actualizado_en = ?8 WHERE id = ?1",
    )
    .bind(reserva_id)
    .bind(input.cliente_id)
    .bind(input.fecha_inicio)
    .bind(input.fecha_fin)
    .bind(input.estado.to_string())
    .bind(input.tarifa_total)
    .bind(&input.notas)
    .bind(ahora)
    .execute(&mut *tx)
    .await?;

    // Recrear asociaciones
    sqlx::query("DELETE FROM reserva_habitaciones WHERE reserva_id = ?1")
        .bind(reserva_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM reserva_huespedes WHERE reserva_id = ?1")
        .bind(reserva_id)
        .execute(&mut *tx)
        .await?;
    for hab_id in &input.habitacion_ids {
        sqlx::query(
            "INSERT INTO reserva_habitaciones (reserva_id, habitacion_id) VALUES (?1, ?2)",
        )
        .bind(reserva_id)
        .bind(hab_id)
        .execute(&mut *tx)
        .await?;
    }
    for huesped_id in &input.huesped_ids {
        sqlx::query("INSERT INTO reserva_huespedes (reserva_id, huesped_id) VALUES (?1, ?2)")
            .bind(reserva_id)
            .bind(huesped_id)
            .execute(&mut *tx)
            .await?;
    }

    auditoria::registrar_movimiento(
        &mut *tx,
        TipoMovimiento::Modificacion,
        "reserva",
        reserva_id,
        Some(&antes),
        Some(input),
        usuario,
    )
    .await?;

    tx.commit().await?;

    post_mutacion(state, reserva_id, AccionSync::Actualizar).await;
    repos::reservas::detalle(&state.db, reserva_id).await
}

/// Cancela una reserva (cambia su estado a 'cancelada').
pub async fn cancelar_reserva(
    state: &Arc<AppState>,
    reserva_id: i64,
    usuario: &str,
) -> AppResult<ReservaDetalle> {
    let antes = repos::reservas::detalle(&state.db, reserva_id).await?;
    if antes.reserva.estado == EstadoReserva::Cancelada {
        return Err(AppError::Validacion(
            "la reserva ya está cancelada".to_string(),
        ));
    }

    let ahora = Local::now().naive_local();
    let mut tx = state.db.begin().await?;

    sqlx::query("UPDATE reservas SET estado = 'cancelada', actualizado_en = ?2 WHERE id = ?1")
        .bind(reserva_id)
        .bind(ahora)
        .execute(&mut *tx)
        .await?;

    auditoria::registrar_movimiento(
        &mut *tx,
        TipoMovimiento::Cancelacion,
        "reserva",
        reserva_id,
        Some(&antes),
        None::<&Reserva>,
        usuario,
    )
    .await?;

    tx.commit().await?;

    post_mutacion(state, reserva_id, AccionSync::Cancelar).await;
    repos::reservas::detalle(&state.db, reserva_id).await
}
