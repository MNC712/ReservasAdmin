//! Servicio de auditoría: registra en `movimientos` el antes/después (JSON)
//! de cada operación relevante. Acepta cualquier executor para poder
//! participar en la misma transacción de la operación auditada.

use chrono::Local;
use serde::Serialize;
use sqlx::Sqlite;

use crate::error::AppResult;
use crate::models::movimiento::TipoMovimiento;

/// Registra un movimiento de auditoría.
/// `antes`/`despues` se serializan a JSON (cadena vacía si son None).
pub async fn registrar_movimiento<'e, E, A, D>(
    executor: E,
    tipo: TipoMovimiento,
    entidad: &str,
    id_entidad: i64,
    antes: Option<&A>,
    despues: Option<&D>,
    usuario: &str,
) -> AppResult<()>
where
    E: sqlx::Executor<'e, Database = Sqlite>,
    A: Serialize + ?Sized,
    D: Serialize + ?Sized,
{
    let json_antes = antes
        .map(|a| serde_json::to_string(a).unwrap_or_default())
        .unwrap_or_default();
    let json_despues = despues
        .map(|d| serde_json::to_string(d).unwrap_or_default())
        .unwrap_or_default();
    let ahora = Local::now().naive_local();

    sqlx::query(
        "INSERT INTO movimientos (tipo, entidad, id_entidad, datos_antes, datos_despues, usuario, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(tipo.to_string())
    .bind(entidad)
    .bind(id_entidad)
    .bind(json_antes)
    .bind(json_despues)
    .bind(usuario)
    .bind(ahora)
    .execute(executor)
    .await?;
    Ok(())
}
