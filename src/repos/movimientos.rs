//! Repositorio de movimientos (auditoría).

use chrono::NaiveDate;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use crate::error::AppResult;
use crate::models::Movimiento;

/// Filtros para el listado de movimientos.
#[derive(Debug, Default, Clone)]
pub struct FiltroMovimientos {
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
    /// 'creacion' | 'modificacion' | 'cancelacion'
    pub tipo: Option<String>,
    /// Texto libre: entidad o usuario
    pub q: Option<String>,
}

/// Lista movimientos (más recientes primero) con filtros dinámicos.
pub async fn listar(db: &SqlitePool, filtro: &FiltroMovimientos) -> AppResult<Vec<Movimiento>> {
    let mut qb: QueryBuilder<Sqlite> =
        QueryBuilder::new("SELECT * FROM movimientos WHERE 1 = 1");

    if let Some(desde) = &filtro.desde {
        qb.push(" AND timestamp >= ")
            .push_bind(desde.and_hms_opt(0, 0, 0).unwrap_or_default());
    }
    if let Some(hasta) = &filtro.hasta {
        qb.push(" AND timestamp <= ")
            .push_bind(hasta.and_hms_opt(23, 59, 59).unwrap_or_default());
    }
    if let Some(tipo) = &filtro.tipo {
        qb.push(" AND tipo = ").push_bind(tipo.clone());
    }
    if let Some(q) = &filtro.q {
        let like = format!("%{}%", q.trim());
        qb.push(" AND (entidad LIKE ")
            .push_bind(like.clone())
            .push(" OR usuario LIKE ")
            .push_bind(like)
            .push(")");
    }
    qb.push(" ORDER BY timestamp DESC, id DESC LIMIT 500");

    Ok(qb.build_query_as::<Movimiento>().fetch_all(db).await?)
}
