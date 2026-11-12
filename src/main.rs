//! Punto de entrada del binario único:
//! 1. Crea el runtime tokio multi-hilo.
//! 2. Inicializa el pool SQLite y ejecuta las migraciones.
//! 3. Lanza en segundo plano: servidor HTTP axum, job de backups y job de polling HostelMate.
//! 4. Arranca la UI de escritorio eframe/egui en el hilo principal.

use hotel_reservas::api;
use hotel_reservas::config::AppConfig;
use hotel_reservas::services::backup;
use hotel_reservas::state::AppState;
use hotel_reservas::ui::app::HotelApp;

fn main() -> eframe::Result<()> {
    // Variables de entorno desde .env (si existe)
    dotenvy::dotenv().ok();

    // Logging con filtro por variable RUST_LOG (por defecto "info")
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = AppConfig::from_env();
    tracing::info!("Iniciando Gestión de Reservas - Hotel");

    // Runtime tokio multi-hilo para toda la lógica async
    let runtime = tokio::runtime::Runtime::new().expect("no se pudo crear el runtime tokio");
    let _guard = runtime.enter();

    // Pool SQLite + migraciones
    let state = runtime
        .block_on(AppState::inicializar(config))
        .expect("no se pudo inicializar la base de datos");

    // Tareas en segundo plano
    runtime.spawn(api::servir(state.clone()));
    runtime.spawn(backup::job_backup(state.clone()));
    runtime.spawn(hotel_reservas::hostelmate::job_polling(state.clone()));

    let handle = runtime.handle().clone();

    // UI de escritorio (bloquea el hilo principal hasta cerrar la ventana)
    let opciones = eframe::NativeOptions {
        // Backend wgpu (DirectX 12): evita el fallo de glutin/WGL en equipos sin OpenGL ES
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 760.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Gestión de Reservas - Hotel",
        opciones,
        Box::new(move |cc| Ok(Box::new(HotelApp::new(cc, handle, state)))),
    )
}
