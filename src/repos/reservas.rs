//! Repositorio de reservas: consultas, detalle y comprobación de solapamiento.

use chrono::NaiveDate;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

use crate::error::{AppError, AppResult};
use crate::models::{Cliente, EstadoReserva, Habitacion, Huesped, Reserva, ReservaDetalle};

/// Filtros opcionales para el listado de reservas.
#[derive(Debug, Default, Clone)]
pub struct FiltroReservas {
    /// Texto libre: nombre/apellidos/documento del titular
    pub q: Option<String>,
    pub estado: Option<EstadoReserva>,
    /// Reservas que terminan después de esta fecha (rango inferior)
    pub desde: Option<NaiveDate>,
    /// Reservas que empiezan antes de esta fecha (rango superior)
    pub hasta: Option<NaiveDate>,
    pub habitacion_id: Option<i64>,
}

/// Lista reservas aplicando filtros dinámicos (QueryBuilder, sin macros).
pub async fn listar(db: &SqlitePool, filtro: &FiltroReservas) -> AppResult<Vec<Reserva>> {
    let mut qb: QueryBuilder<Sqlite> = QueryBuilder::new(
        "SELECT r.* FROM reservas r JOIN clientes c ON c.id = r.cliente_id WHERE 1 = 1",
    );

    if let Some(q) = &filtro.q {
        let like = format!("%{}%", q.trim());
        qb.push(" AND (c.nombre LIKE ")
            .push_bind(like.clone())
            .push(" OR c.apellidos LIKE ")
            .push_bind(like.clone())
            .push(" OR c.documento LIKE ")
            .push_bind(like)
            .push(")");
    }
    if let Some(estado) = &filtro.estado {
        qb.push(" AND r.estado = ").push_bind(estado.to_string());
    }
    if let Some(desde) = &filtro.desde {
        qb.push(" AND r.fecha_fin > ").push_bind(*desde);
    }
    if let Some(hasta) = &filtro.hasta {
        qb.push(" AND r.fecha_inicio < ").push_bind(*hasta);
    }
    if let Some(hab_id) = filtro.habitacion_id {
        qb.push(
            " AND EXISTS (SELECT 1 FROM reserva_habitaciones rh WHERE rh.reserva_id = r.id AND rh.habitacion_id = ",
        )
        .push_bind(hab_id)
        .push(")");
    }
    qb.push(" ORDER BY r.fecha_inicio DESC, r.id DESC");

    let reservas = qb.build_query_as::<Reserva>().fetch_all(db).await?;
    Ok(reservas)
}

/// Obtiene una reserva por id.
pub async fn obtener(db: &SqlitePool, id: i64) -> AppResult<Reserva> {
    sqlx::query_as::<_, Reserva>("SELECT * FROM reservas WHERE id = ?1")
        .bind(id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| AppError::NoEncontrado(format!("reserva id={}", id)))
}

/// Habitaciones asociadas a una reserva.
pub async fn habitaciones_de(db: &SqlitePool, reserva_id: i64) -> AppResult<Vec<Habitacion>> {
    Ok(sqlx::query_as::<_, Habitacion>(
        "SELECT h.id, h.numero, h.tipo, h.capacidad, h.precio_base, h.activa
         FROM habitaciones h
         JOIN reserva_habitaciones rh ON rh.habitacion_id = h.id
         WHERE rh.reserva_id = ?1
         ORDER BY h.numero",
    )
    .bind(reserva_id)
    .fetch_all(db)
    .await?)
}

/// Huéspedes asociados a una reserva.
pub async fn huespedes_de(db: &SqlitePool, reserva_id: i64) -> AppResult<Vec<Huesped>> {
    Ok(sqlx::query_as::<_, Huesped>(
        "SELECT h.id, h.nombre, h.apellidos, h.documento, h.fecha_nacimiento, h.nacionalidad
         FROM huespedes h
         JOIN reserva_huespedes rh ON rh.huesped_id = h.id
         WHERE rh.reserva_id = ?1
         ORDER BY h.nombre",
    )
    .bind(reserva_id)
    .fetch_all(db)
    .await?)
}

/// Construye el detalle completo (reserva + titular + habitaciones + huéspedes).
pub async fn detalle(db: &SqlitePool, id: i64) -> AppResult<ReservaDetalle> {
    let reserva = obtener(db, id).await?;
    let titular: Cliente = sqlx::query_as::<_, Cliente>(
        "SELECT id, nombre, apellidos, documento, email, telefono FROM clientes WHERE id = ?1",
    )
    .bind(reserva.cliente_id)
    .fetch_one(db)
    .await?;
    let habitaciones = habitaciones_de(db, id).await?;
    let huespedes = huespedes_de(db, id).await?;
    Ok(ReservaDetalle {
        reserva,
        titular,
        habitaciones,
        huespedes,
    })
}

/// Lista reservas con su detalle completo (para tablas de la UI).
pub async fn listar_detalle(
    db: &SqlitePool,
    filtro: &FiltroReservas,
) -> AppResult<Vec<ReservaDetalle>> {
    let reservas = listar(db, filtro).await?;
    let mut detalles = Vec::with_capacity(reservas.len());
    for r in reservas {
        detalles.push(detalle(db, r.id).await?);
    }
    Ok(detalles)
}

/// Comprueba si existe otra reserva (no cancelada) que ocupe la habitación
/// en el rango [inicio, fin). Devuelve el id de la reserva conflictiva.
///
/// Regla de solapamiento de rangos [inicio, fin): `inicio1 < fin2 && inicio2 < fin1`.
pub async fn reserva_solapada(
    db: &SqlitePool,
    habitacion_id: i64,
    excluir_reserva_id: i64, // 0 si es reserva nueva
    inicio: NaiveDate,
    fin: NaiveDate,
) -> AppResult<Option<i64>> {
    let conflicto = sqlx::query_scalar::<_, i64>(
        "SELECT r.id FROM reservas r
         JOIN reserva_habitaciones rh ON rh.reserva_id = r.id
         WHERE rh.habitacion_id = ?1
           AND r.estado != 'cancelada'
           AND r.id != ?2
           AND r.fecha_inicio < ?4
           AND r.fecha_fin > ?3
         LIMIT 1",
    )
    .bind(habitacion_id)
    .bind(excluir_reserva_id)
    .bind(inicio)
    .bind(fin)
    .fetch_optional(db)
    .await?;
    Ok(conflicto)
}
