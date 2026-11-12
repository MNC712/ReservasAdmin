//! Ventana principal de la aplicación de escritorio.

use std::sync::Arc;

use tokio::runtime::Handle;

use crate::state::AppState;
use crate::ui::calendario::CalendarioView;
use crate::ui::configuracion::ConfiguracionView;
use crate::ui::dashboard::DashboardView;
use crate::ui::lista_reservas::ListaReservasView;
use crate::ui::movimientos::MovimientosView;
use crate::ui::nueva_reserva::NuevaReservaView;
use crate::ui::tema;

/// Vistas disponibles en la barra lateral.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Vista {
    Dashboard,
    NuevaReserva,
    ListaReservas,
    Calendario,
    Movimientos,
    Configuracion,
}

/// Aplicación eframe principal.
pub struct HotelApp {
    /// Handle del runtime tokio para llamar a la lógica async
    pub rt: Handle,
    /// Estado compartido (pool SQLite, config, cliente HostelMate)
    pub state: Arc<AppState>,
    vista: Vista,
    /// Nombre del hotel mostrado en la cabecera de la barra lateral
    nombre_hotel: String,
    dashboard: DashboardView,
    nueva_reserva: NuevaReservaView,
    lista_reservas: ListaReservasView,
    calendario: CalendarioView,
    movimientos: MovimientosView,
    configuracion: ConfiguracionView,
}

impl HotelApp {
    pub fn new(cc: &eframe::CreationContext<'_>, rt: Handle, state: Arc<AppState>) -> Self {
        // Tema visual global, aplicado una sola vez al arrancar
        tema::aplicar_tema(&cc.egui_ctx);

        // Título del hotel desde la configuración (fallback genérico)
        let nombre_hotel = rt
            .block_on(crate::repos::config::obtener_hotel(&state.db))
            .map(|h| h.nombre)
            .unwrap_or_default();
        let nombre_hotel = if nombre_hotel.trim().is_empty() {
            "Gestión de Reservas".to_string()
        } else {
            nombre_hotel
        };

        Self {
            rt,
            state,
            vista: Vista::Dashboard,
            nombre_hotel,
            dashboard: DashboardView::default(),
            nueva_reserva: NuevaReservaView::default(),
            lista_reservas: ListaReservasView::default(),
            calendario: CalendarioView::default(),
            movimientos: MovimientosView::default(),
            configuracion: ConfiguracionView::default(),
        }
    }
}

impl eframe::App for HotelApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Barra lateral: fondo blanco con borde derecho sutil ---
        let panel = egui::SidePanel::left("menu_lateral")
            .resizable(false)
            .default_width(190.0)
            .frame(
                egui::Frame::none()
                    .fill(tema::PANEL)
                    .inner_margin(egui::Margin::symmetric(12.0, 16.0)),
            )
            .show(ctx, |ui| {
                // Selección del menú: fondo acento suave (el texto se tiñe de acento)
                ui.visuals_mut().selection.bg_fill = tema::ACENTO_SUAVE;

                ui.label(tema::titulo_seccion(&self.nombre_hotel));
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                let opciones = [
                    (Vista::Dashboard, "Dashboard"),
                    (Vista::NuevaReserva, "Nueva reserva"),
                    (Vista::ListaReservas, "Reservas"),
                    (Vista::Calendario, "Calendario"),
                    (Vista::Movimientos, "Movimientos"),
                    (Vista::Configuracion, "Configuración"),
                ];
                for (vista, etiqueta) in opciones {
                    let seleccionado = self.vista == vista;
                    let texto = egui::RichText::new(etiqueta).color(if seleccionado {
                        tema::ACENTO
                    } else {
                        tema::TEXTO
                    });
                    let respuesta = ui.add_sized(
                        [ui.available_width(), 30.0],
                        egui::SelectableLabel::new(seleccionado, texto),
                    );
                    if respuesta.clicked() {
                        self.vista = vista;
                        // Al entrar en una vista se refrescan sus datos
                        match vista {
                            Vista::Dashboard => self.dashboard.marcar_pendiente(),
                            Vista::ListaReservas => self.lista_reservas.marcar_pendiente(),
                            Vista::Calendario => self.calendario.marcar_pendiente(),
                            Vista::Movimientos => self.movimientos.marcar_pendiente(),
                            Vista::NuevaReserva => self.nueva_reserva.marcar_pendiente(),
                            Vista::Configuracion => self.configuracion.marcar_pendiente(),
                        }
                    }
                    ui.add_space(2.0);
                }

                // Versión anclada abajo con separador
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .small()
                            .color(tema::TEXTO_SEC),
                    );
                    ui.separator();
                });
            });
        // Borde derecho de 1px de la barra lateral
        let rect = panel.response.rect;
        ui_painter_vline(ctx, rect);
        fn ui_painter_vline(ctx: &egui::Context, rect: egui::Rect) {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("borde_menu_lateral"),
            ));
            painter.vline(
                rect.right(),
                rect.y_range(),
                egui::Stroke::new(1.0, tema::BORDE),
            );
        }

        // --- Panel central: fondo de la app con margen uniforme ---
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(tema::BG_APP)
                    .inner_margin(egui::Margin::same(20.0)),
            )
            .show(ctx, |ui| match self.vista {
                Vista::Dashboard => self.dashboard.mostrar(ui, &self.rt, &self.state),
                Vista::NuevaReserva => self.nueva_reserva.mostrar(ui, &self.rt, &self.state),
                Vista::ListaReservas => self.lista_reservas.mostrar(ui, ctx, &self.rt, &self.state),
                Vista::Calendario => self.calendario.mostrar(ui, ctx, &self.rt, &self.state),
                Vista::Movimientos => self.movimientos.mostrar(ui, &self.rt, &self.state),
                Vista::Configuracion => self.configuracion.mostrar(ui, &self.rt, &self.state),
            });
    }
}
