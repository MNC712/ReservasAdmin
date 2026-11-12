//! Repositorio de huéspedes (acompañantes).

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::Huesped;

/// Lista todos los huéspedes ordenados por nombre.
pub async fn listar(db: &SqlitePool) -> AppResult<Vec<Huesped>> {
    let huespedes = sqlx::query_as::<_, Huesped>(
        "SELECT id, nombre, apellidos, documento, fecha_nacimiento, nacionalidad FROM huespedes ORDER BY nombre, apellidos",
    )
    .fetch_all(db)
    .await?;
    Ok(huespedes)
}

/// Obtiene un huésped por id.
pub async fn obtener(db: &SqlitePool, id: i64) -> AppResult<Huesped> {
    let huesped = sqlx::query_as::<_, Huesped>(
        "SELECT id, nombre, apellidos, documento, fecha_nacimiento, nacionalidad FROM huespedes WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NoEncontrado(format!("huésped id={}", id)))?;
    Ok(huesped)
}

/// Crea un huésped y devuelve el registro insertado.
pub async fn crear(
    db: &SqlitePool,
    nombre: &str,
    apellidos: &str,
    documento: &str,
    fecha_nacimiento: &str,
    nacionalidad: &str,
) -> AppResult<Huesped> {
    if nombre.trim().is_empty() {
        return Err(AppError::Validacion(
            "el nombre del huésped es obligatorio".to_string(),
        ));
    }
    let id = sqlx::query(
        "INSERT INTO huespedes (nombre, apellidos, documento, fecha_nacimiento, nacionalidad) VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(nombre)
    .bind(apellidos)
    .bind(documento)
    .bind(fecha_nacimiento)
    .bind(nacionalidad)
    .execute(db)
    .await?
    .last_insert_rowid();
    obtener(db, id).await
}
