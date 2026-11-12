//! Servicio de dashboard: KPIs y alertas operativas del día.

use chrono::{Datelike, Local, NaiveDate};
use serde::Serialize;
use sqlx::SqlitePool;

use crate::error::AppResult;

/// KPIs principales del panel de control.
#[derive(Debug, Clone, Serialize)]
pub struct Kpis {
    /// % de habitaciones activas ocupadas hoy (reservas confirmada/checkin vigentes)
    pub ocupacion_actual: f64,
    /// Llegadas del día (fecha_inicio = hoy, no canceladas)
    pub llegadas_hoy: i64,
    /// Salidas del día (fecha_fin = hoy, no canceladas)
    pub salidas_hoy: i64,
    /// Reservas confirmada/checkin vigentes hoy
    pub reservas_activas: i64,
    /// Suma de tarifas de reservas no canceladas que pisan el mes actual
    pub ingresos_mes: f64,
    /// Moneda configurada en el hotel (para mostrar junto a ingresos)
    pub moneda: String,
}

/// Alerta operativa: reserva confirmada/checkin sin acompañantes registrados.
#[derive(Debug, Clone, Serialize)]
pub struct Alerta {
    pub reserva_id: i64,
    pub titular: String,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin: NaiveDate,
    pub detalle: String,
}

/// Datos completos del dashboard.
#[derive(Debug, Clone, Serialize)]
pub struct Dashboard {
    pub kpis: Kpis,
    pub alertas: Vec<Alerta>,
}

/// Calcula KPIs y alertas a fecha de hoy.
pub async fn obtener_dashboard(db: &SqlitePool) -> AppResult<Dashboard> {
    let hoy = Local::now().date_naive();
    let primer_dia_mes = NaiveDate::from_ymd_opt(hoy.year(), hoy.month(), 1).unwrap_or(hoy);
    let (ano, mes) = if hoy.month() == 12 {
        (hoy.year() + 1, 1)
    } else {
        (hoy.year(), hoy.month() + 1)
    };
    let primer_dia_mes_siguiente = NaiveDate::from_ymd_opt(ano, mes, 1).unwrap_or(hoy);

    // Habitaciones ocupadas hoy (reservas confirmada/checkin vigentes)
    let ocupadas: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT rh.habitacion_id) FROM reserva_habitaciones rh
         JOIN reservas r ON r.id = rh.reserva_id
         WHERE r.estado IN ('confirmada', 'checkin')
           AND r.fecha_inicio <= ?1 AND r.fecha_fin > ?1",
    )
    .bind(hoy)
    .fetch_one(db)
    .await?;

    let total_activas: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM habitaciones WHERE activa = 1")
            .fetch_one(db)
            .await?;

    let ocupacion_actual = if total_activas > 0 {
        (ocupadas as f64 / total_activas as f64) * 100.0
    } else {
        0.0
    };

    let llegadas_hoy: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reservas WHERE fecha_inicio = ?1 AND estado != 'cancelada'",
    )
    .bind(hoy)
    .fetch_one(db)
    .await?;

    let salidas_hoy: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reservas WHERE fecha_fin = ?1 AND estado != 'cancelada'",
    )
    .bind(hoy)
    .fetch_one(db)
    .await?;

    let reservas_activas: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reservas
         WHERE estado IN ('confirmada', 'checkin') AND fecha_inicio <= ?1 AND fecha_fin > ?1",
    )
    .bind(hoy)
    .fetch_one(db)
    .await?;

    // Ingresos estimados del mes: reservas no canceladas que pisan el mes
    let ingresos_mes: Option<f64> = sqlx::query_scalar(
        "SELECT SUM(tarifa_total) FROM reservas
         WHERE estado != 'cancelada' AND fecha_inicio < ?2 AND fecha_fin > ?1",
    )
    .bind(primer_dia_mes)
    .bind(primer_dia_mes_siguiente)
    .fetch_one(db)
    .await?;

    let moneda: String =
        sqlx::query_scalar("SELECT moneda FROM hotel_config WHERE id = 1")
            .fetch_optional(db)
            .await?
            .unwrap_or_else(|| "EUR".to_string());

    // Alertas: reservas confirmada/checkin sin acompañantes registrados
    let filas: Vec<(i64, String, String, NaiveDate, NaiveDate)> = sqlx::query_as(
        "SELECT r.id, c.nombre, c.apellidos, r.fecha_inicio, r.fecha_fin
         FROM reservas r
         JOIN clientes c ON c.id = r.cliente_id
         WHERE r.estado IN ('confirmada', 'checkin')
           AND NOT EXISTS (SELECT 1 FROM reserva_huespedes rh WHERE rh.reserva_id = r.id)
         ORDER BY r.fecha_inicio",
    )
    .fetch_all(db)
    .await?;

    let alertas = filas
        .into_iter()
        .map(|(reserva_id, nombre, apellidos, fecha_inicio, fecha_fin)| Alerta {
            reserva_id,
            titular: format!("{} {}", nombre, apellidos).trim().to_string(),
            fecha_inicio,
            fecha_fin,
            detalle: "Reserva sin acompañantes/huéspedes registrados".to_string(),
        })
        .collect();

    Ok(Dashboard {
        kpis: Kpis {
            ocupacion_actual,
            llegadas_hoy,
            salidas_hoy,
            reservas_activas,
            ingresos_mes: ingresos_mes.unwrap_or(0.0),
            moneda,
        },
        alertas,
    })
}
