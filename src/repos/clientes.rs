//! Repositorio de clientes (titulares de reserva).

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::Cliente;

/// Lista todos los clientes ordenados por nombre.
pub async fn listar(db: &SqlitePool) -> AppResult<Vec<Cliente>> {
    let clientes = sqlx::query_as::<_, Cliente>(
        "SELECT id, nombre, apellidos, documento, email, telefono FROM clientes ORDER BY nombre, apellidos",
    )
    .fetch_all(db)
    .await?;
    Ok(clientes)
}

/// Obtiene un cliente por id.
pub async fn obtener(db: &SqlitePool, id: i64) -> AppResult<Cliente> {
    let cliente = sqlx::query_as::<_, Cliente>(
        "SELECT id, nombre, apellidos, documento, email, telefono FROM clientes WHERE id = ?1",
    )
    .bind(id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::NoEncontrado(format!("cliente id={}", id)))?;
    Ok(cliente)
}

/// Crea un cliente y devuelve el registro insertado.
pub async fn crear(
    db: &SqlitePool,
    nombre: &str,
    apellidos: &str,
    documento: &str,
    email: &str,
    telefono: &str,
) -> AppResult<Cliente> {
    if nombre.trim().is_empty() {
        return Err(AppError::Validacion(
            "el nombre del cliente es obligatorio".to_string(),
        ));
    }
    let id = sqlx::query(
        "INSERT INTO clientes (nombre, apellidos, documento, email, telefono) VALUES (?1, ?2, ?3, ?4, ?5)",
    )
    .bind(nombre)
    .bind(apellidos)
    .bind(documento)
    .bind(email)
    .bind(telefono)
    .execute(db)
    .await?
    .last_insert_rowid();
    obtener(db, id).await
}
