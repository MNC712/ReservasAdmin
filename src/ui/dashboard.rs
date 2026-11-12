//! Vista Dashboard: tarjetas KPI uniformes y alertas operativas.

use std::sync::Arc;

use crate::services::dashboard::{self, Dashboard};
use crate::state::AppState;
use crate::ui::tema;

/// Vista del panel de control con KPIs y alertas.
#[derive(Default)]
pub struct DashboardView {
    datos: Option<Dashboard>,
    error: Option<String>,
    pendiente: bool,
}

impl DashboardView {
    /// Marca la vista para recargar datos en el próximo frame.
    pub fn marcar_pendiente(&mut self) {
        self.pendiente = true;
    }

    pub fn mostrar(&mut self, ui: &mut egui::Ui, rt: &tokio::runtime::Handle, state: &Arc<AppState>) {
        // Carga perezosa de datos
        if self.datos.is_none() || self.pendiente {
            self.pendiente = false;
            match rt.block_on(dashboard::obtener_dashboard(&state.db)) {
                Ok(d) => {
                    self.datos = Some(d);
                    self.error = None;
                }
                Err(e) => self.error = Some(e.to_string()),
            }
        }

        ui.heading("Dashboard");
        ui.add_space(4.0);
        if tema::boton_secundario(ui, "↻ Recargar").clicked() {
            self.pendiente = true;
        }
        if let Some(err) = &self.error {
            ui.colored_label(tema::PELIGRO, err);
            return;
        }
        let Some(datos) = &self.datos else { return };
        let k = &datos.kpis;

        ui.add_space(10.0);
        // Tarjetas KPI uniformes en rejilla de 3 columnas
        let kpis: [(String, String, egui::Color32); 5] = [
            ("Ocupación actual".to_string(), format!("{:.0}%", k.ocupacion_actual), tema::ACENTO),
            ("Llegadas hoy".to_string(), k.llegadas_hoy.to_string(), tema::EXITO),
            ("Salidas hoy".to_string(), k.salidas_hoy.to_string(), tema::AVISO),
            ("Reservas activas".to_string(), k.reservas_activas.to_string(), tema::ACENTO),
            (
                format!("Ingresos mes ({})", k.moneda),
                format!("{:.2}", k.ingresos_mes),
                tema::EXITO,
            ),
        ];
        egui::Grid::new("grid_kpis")
            .num_columns(3)
            .spacing([12.0, 12.0])
            .show(ui, |ui| {
                for (i, (titulo, valor, color)) in kpis.iter().enumerate() {
                    tarjeta_kpi(ui, titulo, valor, *color);
                    if (i + 1) % 3 == 0 {
                        ui.end_row();
                    }
                }
            });

        // Alertas: reservas sin acompañantes
        ui.add_space(16.0);
        tema::card(ui, |ui| {
            ui.label(tema::titulo_seccion("Alertas"));
            ui.add_space(6.0);
            if datos.alertas.is_empty() {
                ui.label(
                    egui::RichText::new(
                        "Sin alertas: todas las reservas vigentes tienen acompañantes registrados.",
                    )
                    .color(tema::TEXTO_SEC),
                );
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for alerta in &datos.alertas {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("⚠").color(tema::AVISO).strong(),
                            );
                            ui.label(format!(
                                "Reserva #{} — {} ({} → {})",
                                alerta.reserva_id,
                                alerta.titular,
                                alerta.fecha_inicio.format("%d/%m/%Y"),
                                alerta.fecha_fin.format("%d/%m/%Y")
                            ));
                            tema::chip_estado(ui, &alerta.detalle, tema::AVISO);
                        });
                        ui.add_space(2.0);
                    }
                });
            }
        });
    }
}

/// Dibuja una tarjeta KPI uniforme: etiqueta pequeña gris arriba y
/// número grande con el color indicado.
fn tarjeta_kpi(ui: &mut egui::Ui, titulo: &str, valor: &str, color: egui::Color32) {
    tema::card(ui, |ui| {
        ui.set_min_width(200.0);
        ui.set_min_height(58.0); // ~90px de alto total con el margen interno
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(titulo).small().color(tema::TEXTO_SEC));
            ui.add_space(4.0);
            ui.label(egui::RichText::new(valor).size(tema::TAM_KPI).strong().color(color));
        });
    });
}
