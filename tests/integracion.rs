//! Tests de integración del servicio de reservas contra SQLite en memoria.
//!
//! Cubren la regla de negocio central: validación de solapamiento de fechas
//! (contiguas NO solapan), cancelación que libera las fechas y auditoría.

use chrono::NaiveDate;
use hotel_reservas::db;
use hotel_reservas::error::AppError;
use hotel_reservas::models::{EstadoReserva, ReservaInput};
use hotel_reservas::services::reservas_service::{self, fechas_solapan};
use hotel_reservas::state::AppState;
use sqlx::sqlite::SqlitePoolOptions;

/// Fecha de marzo de 2026 para los tests.
fn d(dia: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 3, dia).unwrap()
}

// ---------------------------------------------------------------------------
// Test unitario de la regla de solapamiento (casos límite)
// ---------------------------------------------------------------------------

#[test]
fn fechas_solapan_casos_limite() {
    // Solapamientos parciales
    assert!(fechas_solapan(d(1), d(5), d(4), d(8)));
    assert!(fechas_solapan(d(4), d(8), d(1), d(5)));
    // Contención
    assert!(fechas_solapan(d(1), d(10), d(3), d(6)));
    assert!(fechas_solapan(d(3), d(6), d(1), d(10)));
    // Mismo rango
    assert!(fechas_solapan(d(1), d(5), d(1), d(5)));
    // Contiguos NO solapan (check-out = check-in siguiente)
    assert!(!fechas_solapan(d(1), d(5), d(5), d(9)));
    assert!(!fechas_solapan(d(5), d(9), d(1), d(5)));
    // Disjuntos
    assert!(!fechas_solapan(d(1), d(3), d(7), d(9)));
}

// ---------------------------------------------------------------------------
// Infraestructura de tests de integración
// ---------------------------------------------------------------------------

/// Crea un AppState con SQLite en memoria migrado y datos mínimos:
/// un cliente y la primera habitación del seed.
/// IMPORTANTE: pool de UNA sola conexión: con sqlite::memory: cada conexión
/// nueva abriría una base de datos distinta.
async fn estado_test() -> (std::sync::Arc<AppState>, i64, i64) {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("no se pudo abrir SQLite en memoria");
    db::migrar(&pool).await.expect("falló la migración");

    // Cliente de prueba
    let cliente_id: i64 = sqlx::query_scalar(
        "INSERT INTO clientes (nombre, apellidos, documento, email, telefono) VALUES ('Ana', 'García', '12345678', 'ana@test.es', '600000000') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("no se pudo crear el cliente de prueba");

    // Primera habitación del seed
    let habitacion_id: i64 = sqlx::query_scalar("SELECT id FROM habitaciones ORDER BY id LIMIT 1")
        .fetch_one(&pool)
        .await
        .expect("no hay habitaciones en el seed");

    (AppState::for_tests(pool).await, cliente_id, habitacion_id)
}

/// Construye un ReservaInput mínimo.
fn input(cliente_id: i64, habitacion_id: i64, inicio: NaiveDate, fin: NaiveDate) -> ReservaInput {
    ReservaInput {
        cliente_id,
        fecha_inicio: inicio,
        fecha_fin: fin,
        estado: EstadoReserva::Confirmada,
        tarifa_total: 100.0,
        notas: "reserva de test".to_string(),
        habitacion_ids: vec![habitacion_id],
        huesped_ids: vec![],
    }
}

// ---------------------------------------------------------------------------
// Tests de integración
// ---------------------------------------------------------------------------

/// Una reserva que solapa con otra existente debe ser rechazada.
#[tokio::test]
async fn reserva_solapada_es_rechazada() {
    let (state, cliente_id, hab_id) = estado_test().await;

    reservas_service::crear_reserva(&state, &input(cliente_id, hab_id, d(10), d(15)), "test")
        .await
        .expect("la primera reserva debería crearse");

    let resultado =
        reservas_service::crear_reserva(&state, &input(cliente_id, hab_id, d(13), d(20)), "test")
            .await;
    assert!(
        matches!(resultado, Err(AppError::Solapamiento(_))),
        "se esperaba AppError::Solapamiento, se obtuvo: {:?}",
        resultado
    );
}

/// Dos reservas contiguas (una termina cuando empieza la otra) deben admitirse.
#[tokio::test]
async fn reservas_contiguas_no_solapan() {
    let (state, cliente_id, hab_id) = estado_test().await;

    reservas_service::crear_reserva(&state, &input(cliente_id, hab_id, d(10), d(15)), "test")
        .await
        .expect("la primera reserva debería crearse");

    reservas_service::crear_reserva(&state, &input(cliente_id, hab_id, d(15), d(20)), "test")
        .await
        .expect("una reserva contigua (check-in = check-out anterior) debe admitirse");
}

/// Tras cancelar una reserva, sus fechas quedan libres para una nueva.
#[tokio::test]
async fn cancelar_permite_recrear_en_las_mismas_fechas() {
    let (state, cliente_id, hab_id) = estado_test().await;

    let creada =
        reservas_service::crear_reserva(&state, &input(cliente_id, hab_id, d(10), d(15)), "test")
            .await
            .expect("la primera reserva debería crearse");

    reservas_service::cancelar_reserva(&state, creada.reserva.id, "test")
        .await
        .expect("la cancelación debería funcionar");

    reservas_service::crear_reserva(&state, &input(cliente_id, hab_id, d(10), d(15)), "test")
        .await
        .expect("tras cancelar, las mismas fechas deben quedar libres");
}

/// Cada mutación debe quedar registrada en la tabla de movimientos.
#[tokio::test]
async fn la_auditoria_registra_los_movimientos() {
    let (state, cliente_id, hab_id) = estado_test().await;

    let creada =
        reservas_service::crear_reserva(&state, &input(cliente_id, hab_id, d(10), d(15)), "recepcion")
            .await
            .expect("la reserva debería crearse");
    let id = creada.reserva.id;
    reservas_service::cancelar_reserva(&state, id, "recepcion")
        .await
        .expect("la cancelación debería funcionar");

    // Debe haber un movimiento de creación con datos_despues
    let (tipo, datos_despues): (String, String) = sqlx::query_as(
        "SELECT tipo, datos_despues FROM movimientos WHERE entidad = 'reserva' AND id_entidad = ?1 AND tipo = 'creacion'",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .expect("debe existir el movimiento de creación");
    assert_eq!(tipo, "creacion");
    assert!(!datos_despues.is_empty(), "la creación debe registrar datos_despues");

    // Y uno de cancelación con datos_antes
    let datos_antes: String = sqlx::query_scalar(
        "SELECT datos_antes FROM movimientos WHERE entidad = 'reserva' AND id_entidad = ?1 AND tipo = 'cancelacion'",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .expect("debe existir el movimiento de cancelación");
    assert!(!datos_antes.is_empty(), "la cancelación debe registrar datos_antes");

    // El usuario registrado es el que se pasó al servicio
    let usuario: String = sqlx::query_scalar(
        "SELECT usuario FROM movimientos WHERE entidad = 'reserva' AND id_entidad = ?1 LIMIT 1",
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    .unwrap();
    assert_eq!(usuario, "recepcion");
}
