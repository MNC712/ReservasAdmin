//! Creación del pool SQLite y ejecución de migraciones embebidas.

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;

use crate::error::AppResult;

/// Crea el pool de conexiones SQLite.
/// - `create_if_missing(true)`: crea `hotel.db` si no existe.
/// - `foreign_keys(true)`: activa integridad referencial en cada conexión.
pub async fn crear_pool(database_url: &str) -> AppResult<SqlitePool> {
    let opciones = SqliteConnectOptions::from_str(database_url)
        .map_err(crate::error::AppError::Db)?
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opciones)
        .await?;

    Ok(pool)
}

/// Ejecuta las migraciones embebidas del directorio `migrations/`.
pub async fn migrar(pool: &SqlitePool) -> AppResult<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

/// Atajo: crea pool + migra. Usado por main y por los tests.
pub async fn inicializar(database_url: &str) -> AppResult<SqlitePool> {
    let pool = crear_pool(database_url).await?;
    migrar(&pool).await?;
    Ok(pool)
}
