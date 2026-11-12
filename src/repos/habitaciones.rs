//! Repositorio de habitaciones.

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::Habitacion;

const COLUMNAS: &str = "id, numero, tipo, capacidad, precio_base, activa";

/// Lista todas las habitaciones ordenadas por número.
pub async fn listar(db: &SqlitePool) -> AppResult<Vec<Habitacion>> {
    let sql = format!("SELECT {} FROM habitaciones ORDER BY numero", COLUMNAS);
    Ok(sqlx::query_as::<_, Habitacion>(&sql).fetch_all(db).await?)
}

/// Lista solo las habitaciones activas (para el formulario de reservas).
pub async fn listar_activas(db: &SqlitePool) -> AppResult<Vec<Habitacion>> {
    let sql = format!(
        "SELECT {} FROM habitaciones WHERE activa = 1 ORDER BY numero",
        COLUMNAS
    );
    Ok(sqlx::query_as::<_, Habitacion>(&sql).fetch_all(db).await?)
}

/// Obtiene una habitación por id.
pub async fn obtener(db: &SqlitePool, id: i64) -> AppResult<Habitacion> {
    let sql = format!("SELECT {} FROM habitaciones WHERE id = ?1", COLUMNAS);
    sqlx::query_as::<_, Habitacion>(&sql)
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NoEncontrado(format!("habitación id={}", id)))
}

/// Crea una habitación.
pub async fn crear(
    db: &SqlitePool,
    numero: &str,
    tipo: &str,
    capacidad: i64,
    precio_base: f64,
) -> AppResult<Habitacion> {
    if numero.trim().is_empty() {
        return Err(AppError::Validacion(
            "el número de habitación es obligatorio".to_string(),
        ));
    }
    let id = sqlx::query(
        "INSERT INTO habitaciones (numero, tipo, capacidad, precio_base) VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(numero)
    .bind(tipo)
    .bind(capacidad)
    .bind(precio_base)
    .execute(db)
    .await?
    .last_insert_rowid();
    obtener(db, id).await
}

/// Actualiza una habitación existente.
pub async fn actualizar(
    db: &SqlitePool,
    id: i64,
    numero: &str,
    tipo: &str,
    capacidad: i64,
    precio_base: f64,
    activa: bool,
) -> AppResult<Habitacion> {
    sqlx::query(
        "UPDATE habitaciones SET numero = ?2, tipo = ?3, capacidad = ?4, precio_base = ?5, activa = ?6 WHERE id = ?1",
    )
    .bind(id)
    .bind(numero)
    .bind(tipo)
    .bind(capacidad)
    .bind(precio_base)
    .bind(activa)
    .execute(db)
    .await?;
    obtener(db, id).await
}

/// Elimina una habitación. Falla si tiene reservas asociadas (FK).
pub async fn eliminar(db: &SqlitePool, id: i64) -> AppResult<()> {
    let usada = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM reserva_habitaciones WHERE habitacion_id = ?1",
    )
    .bind(id)
    .fetch_one(db)
    .await?;
    if usada > 0 {
        return Err(AppError::Validacion(format!(
            "la habitación id={} tiene reservas asociadas; márquela como inactiva en su lugar",
            id
        )));
    }
    sqlx::query("DELETE FROM habitaciones WHERE id = ?1")
        .bind(id)
        .execute(db)
        .await?;
    Ok(())
}
