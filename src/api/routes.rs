//! Definición del router axum con todas las rutas de la API REST.

use std::sync::Arc;

use axum::{
    routing::{get, post, put},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::api::handlers;
use crate::state::AppState;

/// Construye el router con estado compartido `Arc<AppState>`.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        // Dashboard
        .route("/api/dashboard", get(handlers::dashboard))
        // Reservas
        .route(
            "/api/reservas",
            get(handlers::listar_reservas).post(handlers::crear_reserva),
        )
        .route(
            "/api/reservas/:id",
            get(handlers::obtener_reserva)
                .put(handlers::actualizar_reserva)
                .delete(handlers::cancelar_reserva),
        )
        .route("/api/reservas/:id/pdf", post(handlers::generar_pdf))
        // Movimientos (auditoría)
        .route("/api/movimientos", get(handlers::listar_movimientos))
        // Habitaciones
        .route(
            "/api/habitaciones",
            get(handlers::listar_habitaciones).post(handlers::crear_habitacion),
        )
        .route(
            "/api/habitaciones/:id",
            put(handlers::actualizar_habitacion).delete(handlers::eliminar_habitacion),
        )
        // Clientes y huéspedes (auxiliares para la UI)
        .route(
            "/api/clientes",
            get(handlers::listar_clientes).post(handlers::crear_cliente),
        )
        .route(
            "/api/huespedes",
            get(handlers::listar_huespedes).post(handlers::crear_huesped),
        )
        // Configuración
        .route(
            "/api/config/hotel",
            get(handlers::obtener_config_hotel).put(handlers::actualizar_config_hotel),
        )
        .route(
            "/api/config/colores",
            get(handlers::listar_colores).put(handlers::actualizar_color),
        )
        .route(
            "/api/config/backup",
            get(handlers::obtener_config_backup).put(handlers::actualizar_config_backup),
        )
        // Importación Excel y webhook HostelMate
        .route("/api/import/excel", post(handlers::importar_excel))
        .route("/api/webhooks/hostelmate", post(handlers::webhook_hostelmate))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Arranca el servidor HTTP en `SERVER_ADDR` (por defecto 127.0.0.1:3000).
pub async fn servir(state: Arc<AppState>) -> anyhow::Result<()> {
    let addr = state.config.server_addr.clone();
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("API HTTP escuchando en http://{}", addr);
    axum::serve(listener, router(state)).await?;
    Ok(())
}
