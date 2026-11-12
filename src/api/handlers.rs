//! Handlers finos de la API: extraen parámetros, delegan en servicios/repos
//! y devuelven JSON. Los errores usan `AppError` (IntoResponse).

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::error::AppResult;
use crate::hostelmate::{upsert_reserva_externa, ReservaExterna};
use crate::models::{
    ColorEstado, ConfigBackup, EstadoReserva, HotelConfig, ReservaInput,
};
use crate::repos;
use crate::repos::movimientos::FiltroMovimientos;
use crate::repos::reservas::FiltroReservas;
use crate::services::{backup, dashboard, reservas_service};
use crate::state::AppState;

// ---------------- Dashboard ----------------

pub async fn dashboard(State(state): State<Arc<AppState>>) -> AppResult<Json<serde_json::Value>> {
    let datos = dashboard::obtener_dashboard(&state.db).await?;
    Ok(Json(serde_json::to_value(datos).unwrap_or_default()))
}

// ---------------- Reservas ----------------

/// Query params del listado de reservas.
#[derive(Debug, Deserialize)]
pub struct QueryReservas {
    pub q: Option<String>,
    pub estado: Option<String>,
    pub desde: Option<chrono::NaiveDate>,
    pub hasta: Option<chrono::NaiveDate>,
    pub habitacion_id: Option<i64>,
}

pub async fn listar_reservas(
    State(state): State<Arc<AppState>>,
    Query(q): Query<QueryReservas>,
) -> AppResult<Json<serde_json::Value>> {
    let estado = match &q.estado {
        Some(e) if !e.is_empty() => Some(e.parse::<EstadoReserva>()?),
        _ => None,
    };
    let filtro = FiltroReservas {
        q: q.q,
        estado,
        desde: q.desde,
        hasta: q.hasta,
        habitacion_id: q.habitacion_id,
    };
    let reservas = repos::reservas::listar_detalle(&state.db, &filtro).await?;
    Ok(Json(serde_json::to_value(reservas).unwrap_or_default()))
}

pub async fn obtener_reserva(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let det = repos::reservas::detalle(&state.db, id).await?;
    Ok(Json(serde_json::to_value(det).unwrap_or_default()))
}

pub async fn crear_reserva(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ReservaInput>,
) -> AppResult<Json<serde_json::Value>> {
    let det = reservas_service::crear_reserva(&state, &input, "api").await?;
    Ok(Json(serde_json::to_value(det).unwrap_or_default()))
}

pub async fn actualizar_reserva(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(input): Json<ReservaInput>,
) -> AppResult<Json<serde_json::Value>> {
    let det = reservas_service::actualizar_reserva(&state, id, &input, "api").await?;
    Ok(Json(serde_json::to_value(det).unwrap_or_default()))
}

/// DELETE /api/reservas/:id -> cancela la reserva (no borra el registro).
pub async fn cancelar_reserva(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let det = reservas_service::cancelar_reserva(&state, id, "api").await?;
    Ok(Json(serde_json::to_value(det).unwrap_or_default()))
}

/// Genera el comprobante PDF y devuelve la ruta del archivo.
pub async fn generar_pdf(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    let ruta = crate::pdf::comprobante::generar_comprobante(&state.db, &state.config, id).await?;
    Ok(Json(serde_json::json!({ "ruta": ruta.to_string_lossy() })))
}

// ---------------- Movimientos ----------------

/// Query params del listado de movimientos.
#[derive(Debug, Deserialize)]
pub struct QueryMovimientos {
    pub desde: Option<chrono::NaiveDate>,
    pub hasta: Option<chrono::NaiveDate>,
    pub tipo: Option<String>,
    pub q: Option<String>,
}

pub async fn listar_movimientos(
    State(state): State<Arc<AppState>>,
    Query(q): Query<QueryMovimientos>,
) -> AppResult<Json<serde_json::Value>> {
    let filtro = FiltroMovimientos {
        desde: q.desde,
        hasta: q.hasta,
        tipo: q.tipo.filter(|t| !t.is_empty()),
        q: q.q,
    };
    let movimientos = repos::movimientos::listar(&state.db, &filtro).await?;
    Ok(Json(serde_json::to_value(movimientos).unwrap_or_default()))
}

// ---------------- Habitaciones ----------------

#[derive(Debug, Deserialize)]
pub struct HabitacionInput {
    pub numero: String,
    pub tipo: String,
    #[serde(default = "capacidad_por_defecto")]
    pub capacidad: i64,
    #[serde(default)]
    pub precio_base: f64,
    #[serde(default = "verdadero")]
    pub activa: bool,
}

fn capacidad_por_defecto() -> i64 {
    2
}
fn verdadero() -> bool {
    true
}

pub async fn listar_habitaciones(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<serde_json::Value>> {
    let habs = repos::habitaciones::listar(&state.db).await?;
    Ok(Json(serde_json::to_value(habs).unwrap_or_default()))
}

pub async fn crear_habitacion(
    State(state): State<Arc<AppState>>,
    Json(input): Json<HabitacionInput>,
) -> AppResult<Json<serde_json::Value>> {
    let hab = repos::habitaciones::crear(
        &state.db,
        &input.numero,
        &input.tipo,
        input.capacidad,
        input.precio_base,
    )
    .await?;
    Ok(Json(serde_json::to_value(hab).unwrap_or_default()))
}

pub async fn actualizar_habitacion(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(input): Json<HabitacionInput>,
) -> AppResult<Json<serde_json::Value>> {
    let hab = repos::habitaciones::actualizar(
        &state.db,
        id,
        &input.numero,
        &input.tipo,
        input.capacidad,
        input.precio_base,
        input.activa,
    )
    .await?;
    Ok(Json(serde_json::to_value(hab).unwrap_or_default()))
}

pub async fn eliminar_habitacion(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> AppResult<Json<serde_json::Value>> {
    repos::habitaciones::eliminar(&state.db, id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ---------------- Clientes y huéspedes ----------------

#[derive(Debug, Deserialize)]
pub struct ClienteInput {
    pub nombre: String,
    #[serde(default)]
    pub apellidos: String,
    #[serde(default)]
    pub documento: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub telefono: String,
}

pub async fn listar_clientes(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<serde_json::Value>> {
    let clientes = repos::clientes::listar(&state.db).await?;
    Ok(Json(serde_json::to_value(clientes).unwrap_or_default()))
}

pub async fn crear_cliente(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ClienteInput>,
) -> AppResult<Json<serde_json::Value>> {
    let c = repos::clientes::crear(
        &state.db,
        &input.nombre,
        &input.apellidos,
        &input.documento,
        &input.email,
        &input.telefono,
    )
    .await?;
    Ok(Json(serde_json::to_value(c).unwrap_or_default()))
}

#[derive(Debug, Deserialize)]
pub struct HuespedInput {
    pub nombre: String,
    #[serde(default)]
    pub apellidos: String,
    #[serde(default)]
    pub documento: String,
    #[serde(default)]
    pub fecha_nacimiento: String,
    #[serde(default)]
    pub nacionalidad: String,
}

pub async fn listar_huespedes(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<serde_json::Value>> {
    let huespedes = repos::huespedes::listar(&state.db).await?;
    Ok(Json(serde_json::to_value(huespedes).unwrap_or_default()))
}

pub async fn crear_huesped(
    State(state): State<Arc<AppState>>,
    Json(input): Json<HuespedInput>,
) -> AppResult<Json<serde_json::Value>> {
    let h = repos::huespedes::crear(
        &state.db,
        &input.nombre,
        &input.apellidos,
        &input.documento,
        &input.fecha_nacimiento,
        &input.nacionalidad,
    )
    .await?;
    Ok(Json(serde_json::to_value(h).unwrap_or_default()))
}

// ---------------- Configuración ----------------

pub async fn obtener_config_hotel(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<serde_json::Value>> {
    let cfg = repos::config::obtener_hotel(&state.db).await?;
    Ok(Json(serde_json::to_value(cfg).unwrap_or_default()))
}

pub async fn actualizar_config_hotel(
    State(state): State<Arc<AppState>>,
    Json(cfg): Json<HotelConfig>,
) -> AppResult<Json<serde_json::Value>> {
    repos::config::actualizar_hotel(&state.db, &cfg).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn listar_colores(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<serde_json::Value>> {
    let colores = repos::config::listar_colores(&state.db).await?;
    Ok(Json(serde_json::to_value(colores).unwrap_or_default()))
}

pub async fn actualizar_color(
    State(state): State<Arc<AppState>>,
    Json(color): Json<ColorEstado>,
) -> AppResult<Json<serde_json::Value>> {
    repos::config::actualizar_color(&state.db, &color.estado, &color.color_hex).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn obtener_config_backup(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<serde_json::Value>> {
    let cfg = repos::config::obtener_backup(&state.db).await?;
    Ok(Json(serde_json::to_value(cfg).unwrap_or_default()))
}

pub async fn actualizar_config_backup(
    State(state): State<Arc<AppState>>,
    Json(cfg): Json<ConfigBackup>,
) -> AppResult<Json<serde_json::Value>> {
    repos::config::actualizar_backup(&state.db, &cfg).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ---------------- Importación Excel ----------------

#[derive(Debug, Deserialize)]
pub struct ImportExcelInput {
    /// Ruta del archivo .xlsx a importar
    pub ruta: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ImportExcelResultado {
    pub importadas: usize,
    pub errores: Vec<String>,
}

pub async fn importar_excel(
    State(state): State<Arc<AppState>>,
    Json(input): Json<ImportExcelInput>,
) -> AppResult<Json<serde_json::Value>> {
    let importadas = crate::excel::import::importar_reservas(&state.db, &input.ruta).await?;
    let resultado =
        crate::excel::import::insertar_importadas(&state, importadas).await?;
    Ok(Json(serde_json::json!({
        "importadas": resultado.0,
        "errores": resultado.1,
    })))
}

// ---------------- Webhook HostelMate ----------------

/// Recibe una reserva externa vía webhook (alternativa push al polling).
/// TODO(HostelMate): validar la firma del webhook (cabecera HMAC o secreto)
/// según la documentación oficial antes de exponerlo a Internet.
pub async fn webhook_hostelmate(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ReservaExterna>,
) -> AppResult<Json<serde_json::Value>> {
    upsert_reserva_externa(&state, &payload).await?;
    if let Err(e) =
        crate::excel::export::exportar_reservas(&state.db, &state.config.excel_backup_path).await
    {
        tracing::error!("error reexportando Excel tras webhook: {}", e);
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ---------------- Backup manual ----------------

/// Ejecuta un backup inmediato (también disponible desde la UI).
pub async fn ejecutar_backup(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<serde_json::Value>> {
    let carpeta = backup::ejecutar_backup_ahora(&state).await?;
    Ok(Json(serde_json::json!({ "carpeta": carpeta.to_string_lossy() })))
}
