//! Vista Calendario: ocupación por habitación y día, estilo hoja Excel.
//! Se dibuja con painter manual: rectángulos coloreados por estado de reserva.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Datelike, Duration, Local, NaiveDate};

use crate::models::{ColorEstado, EstadoReserva, Habitacion, ReservaDetalle};
use crate::repos;
use crate::state::AppState;
use crate::ui::tema;

/// Número de días visibles en el calendario.
const DIAS: i64 = 30;
/// Tamaño de cada celda día (ancho x alto).
const CELDA: egui::Vec2 = egui::vec2(90.0, 28.0);
/// Ancho de la columna de números de habitación.
const ANCHO_HABITACION: f32 = 110.0;

/// Vista del calendario de ocupación.
pub struct CalendarioView {
    fecha_base: NaiveDate,
    habitaciones: Vec<Habitacion>,
    reservas: Vec<ReservaDetalle>,
    colores: Vec<ColorEstado>,
    pendiente: bool,
    mensaje: Option<String>,
    /// Reserva seleccionada al hacer clic en una celda ocupada.
    detalle_abierto: Option<ReservaDetalle>,
}

impl Default for CalendarioView {
    fn default() -> Self {
        Self {
            fecha_base: Local::now().date_naive(),
            habitaciones: Vec::new(),
            reservas: Vec::new(),
            colores: Vec::new(),
            pendiente: true,
            mensaje: None,
            detalle_abierto: None,
        }
    }
}

impl CalendarioView {
    pub fn marcar_pendiente(&mut self) {
        self.pendiente = true;
    }

    fn cargar(&mut self, rt: &tokio::runtime::Handle, state: &Arc<AppState>) {
        let hasta = self.fecha_base + Duration::days(DIAS);
        let filtro = repos::reservas::FiltroReservas {
            desde: Some(self.fecha_base),
            hasta: Some(hasta),
            ..Default::default()
        };
        let res = rt.block_on(async {
            Ok::<_, crate::error::AppError>((
                repos::habitaciones::listar(&state.db).await?,
                repos::reservas::listar_detalle(&state.db, &filtro).await?,
                repos::config::listar_colores(&state.db).await?,
            ))
        });
        match res {
            Ok((h, r, c)) => {
                self.habitaciones = h;
                self.reservas = r;
                self.colores = c;
                self.mensaje = None;
            }
            Err(e) => self.mensaje = Some(e.to_string()),
        }
    }

    /// Color configurado para un estado (fallback gris).
    fn color_estado(&self, estado: &EstadoReserva) -> egui::Color32 {
        self.colores
            .iter()
            .find(|c| c.estado == estado.to_string())
            .map(|c| c.a_color32())
            .unwrap_or(egui::Color32::LIGHT_GRAY)
    }

    /// Construye un índice (fila_habitacion, dia_offset) → índice de reserva
    /// para saber qué reserva ocupa cada celda.
    fn indice_ocupacion(&self) -> HashMap<(usize, i64), usize> {
        let mut mapa = HashMap::new();
        for (idx, det) in self.reservas.iter().enumerate() {
            let r = &det.reserva;
            if r.estado == EstadoReserva::Cancelada {
                continue;
            }
            for (fila, hab) in self.habitaciones.iter().enumerate() {
                if !det.habitaciones.iter().any(|h| h.id == hab.id) {
                    continue;
                }
                let mut d = r.fecha_inicio.max(self.fecha_base);
                while d < r.fecha_fin && d < self.fecha_base + Duration::days(DIAS) {
                    let offset = (d - self.fecha_base).num_days();
                    mapa.insert((fila, offset), idx);
                    d += Duration::days(1);
                }
            }
        }
        mapa
    }

    pub fn mostrar(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rt: &tokio::runtime::Handle,
        state: &Arc<AppState>,
    ) {
        if self.pendiente {
            self.pendiente = false;
            self.cargar(rt, state);
        }

        ui.heading("Calendario de ocupación");
        ui.add_space(4.0);

        // Barra de navegación de fechas
        ui.horizontal(|ui| {
            if tema::boton_secundario(ui, "◀ Semana anterior").clicked() {
                self.fecha_base -= Duration::days(7);
                self.pendiente = true;
            }
            if tema::boton_secundario(ui, "Hoy").clicked() {
                self.fecha_base = Local::now().date_naive();
                self.pendiente = true;
            }
            if tema::boton_secundario(ui, "Semana siguiente ▶").clicked() {
                self.fecha_base += Duration::days(7);
                self.pendiente = true;
            }
            ui.label(
                egui::RichText::new(format!(
                    "Mostrando {} días desde {}",
                    DIAS,
                    self.fecha_base.format("%d/%m/%Y")
                ))
                .color(tema::TEXTO_SEC),
            );
        });
        if let Some(m) = &self.mensaje {
            ui.colored_label(tema::PELIGRO, m);
        }

        // Leyenda de colores por estado
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Leyenda:").small().color(tema::TEXTO_SEC));
            for c in &self.colores {
                tema::chip_estado(ui, &c.estado, c.a_color32());
            }
        });
        ui.add_space(4.0);

        let hoy = Local::now().date_naive();
        let ocupacion = self.indice_ocupacion();
        let filas = self.habitaciones.len();
        let fecha_base = self.fecha_base;

        egui::ScrollArea::both().show(ui, |ui| {
            let ancho_total = ANCHO_HABITACION + CELDA.x * DIAS as f32;
            let alto_total = CELDA.y * (filas + 1) as f32; // +1 cabecera
            let (respuesta, painter) =
                ui.allocate_painter(egui::vec2(ancho_total, alto_total), egui::Sense::click());
            let origen = respuesta.rect.min;

            // Cabecera de días
            let dias_semana = ["lu", "ma", "mi", "ju", "vi", "sá", "do"];
            for dia in 0..DIAS {
                let fecha = fecha_base + Duration::days(dia);
                let x = origen.x + ANCHO_HABITACION + dia as f32 * CELDA.x;
                let rect = egui::Rect::from_min_size(egui::pos2(x, origen.y), CELDA);
                // Cabeceras con fondo gris muy claro y texto pequeño en negrita
                painter.rect_filled(rect, 0.0, tema::FAINT);
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    format!(
                        "{}\n{}",
                        dias_semana[fecha.weekday().num_days_from_monday() as usize],
                        fecha.format("%d/%m")
                    ),
                    egui::FontId::new(10.0, egui::FontFamily::Proportional),
                    tema::TEXTO_SEC,
                );
                painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, tema::BORDE));
            }

            // Filas por habitación
            for (fila, hab) in self.habitaciones.iter().enumerate() {
                let y = origen.y + CELDA.y * (fila + 1) as f32;
                // Celda con el número de habitación: fondo blanco y texto secundario
                let rect_hab = egui::Rect::from_min_size(
                    egui::pos2(origen.x, y),
                    egui::vec2(ANCHO_HABITACION, CELDA.y),
                );
                painter.rect_filled(rect_hab, 0.0, tema::PANEL);
                painter.text(
                    rect_hab.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{} ({})", hab.numero, hab.tipo),
                    egui::FontId::proportional(11.0),
                    tema::TEXTO_SEC,
                );
                painter.rect_stroke(rect_hab, 0.0, egui::Stroke::new(1.0, tema::BORDE));

                // Celdas de días
                for dia in 0..DIAS {
                    let x = origen.x + ANCHO_HABITACION + dia as f32 * CELDA.x;
                    let rect = egui::Rect::from_min_size(egui::pos2(x, y), CELDA);
                    let fecha = fecha_base + Duration::days(dia);
                    let es_finde = matches!(fecha.weekday().num_days_from_monday(), 5 | 6);
                    let color_fondo = if let Some(&idx) = ocupacion.get(&(fila, dia)) {
                        let det = &self.reservas[idx];
                        // Color de estado al 25% de alpha sobre el fondo
                        self.color_estado(&det.reserva.estado).linear_multiply(0.25)
                    } else if es_finde {
                        tema::FAINT
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    painter.rect_filled(rect, 0.0, color_fondo);
                    // Rejilla sutil; el día de hoy lleva borde acentuado de 2px
                    if fecha == hoy {
                        painter.rect_stroke(rect, 0.0, egui::Stroke::new(2.0, tema::ACENTO));
                    } else {
                        painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, tema::BORDE));
                    }

                    // Nombre del titular solo en el primer día visible del tramo
                    if let Some(&idx) = ocupacion.get(&(fila, dia)) {
                        let det = &self.reservas[idx];
                        let primer_dia_visible = det.reserva.fecha_inicio.max(fecha_base);
                        if fecha == primer_dia_visible {
                            painter.text(
                                rect.left_center() + egui::vec2(3.0, 0.0),
                                egui::Align2::LEFT_CENTER,
                                det.titular.nombre_completo(),
                                egui::FontId::proportional(10.0),
                                tema::TEXTO,
                            );
                        }
                    }
                }
            }

            // Clic en una celda: abrir detalle de la reserva
            if respuesta.clicked() {
                if let Some(pos) = respuesta.interact_pointer_pos() {
                    let dx = pos.x - origen.x - ANCHO_HABITACION;
                    let dy = pos.y - origen.y - CELDA.y;
                    if dx >= 0.0 && dy >= 0.0 {
                        let dia = (dx / CELDA.x) as i64;
                        let fila = (dy / CELDA.y) as usize;
                        if let Some(&idx) = ocupacion.get(&(fila, dia)) {
                            self.detalle_abierto = Some(self.reservas[idx].clone());
                        }
                    }
                }
            }
        });

        // Ventana de detalle al hacer clic
        if let Some(det) = self.detalle_abierto.clone() {
            let mut abierta = true;
            egui::Window::new(format!("Reserva #{}", det.reserva.id))
                .collapsible(false)
                .open(&mut abierta)
                .show(ctx, |ui| {
                    ui.label(format!("Titular: {}", det.titular.nombre_completo()));
                    ui.label(format!(
                        "Fechas: {} → {}",
                        det.reserva.fecha_inicio.format("%d/%m/%Y"),
                        det.reserva.fecha_fin.format("%d/%m/%Y")
                    ));
                    ui.label(format!("Estado: {}", det.reserva.estado.etiqueta()));
                    ui.label(format!("Habitaciones: {}", det.habitaciones_resumen()));
                    ui.label(format!("Total: {:.2}", det.reserva.tarifa_total));
                    if !det.reserva.notas.is_empty() {
                        ui.label(format!("Notas: {}", det.reserva.notas));
                    }
                });
            if !abierta {
                self.detalle_abierto = None;
            }
        }
    }
}
