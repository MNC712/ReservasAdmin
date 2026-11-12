//! Vista Movimientos: log de auditoría con filtros y JSON expandible.

use std::collections::HashSet;
use std::sync::Arc;

use chrono::NaiveDate;
use egui_extras::DatePickerButton;

use crate::models::Movimiento;
use crate::repos;
use crate::state::AppState;
use crate::ui::tema;

/// Tipos de movimiento disponibles para el filtro (valor persistido en BD).
const TIPOS: [&str; 3] = ["creacion", "modificacion", "cancelacion"];

/// Vista del log de movimientos/auditoría.
#[derive(Default)]
pub struct MovimientosView {
    movimientos: Vec<Movimiento>,
    filtros: repos::movimientos::FiltroMovimientos,
    /// Texto libre del filtro (se pasa a BD como Option<String>)
    q_texto: String,
    usar_desde: bool,
    usar_hasta: bool,
    fecha_desde: NaiveDate,
    fecha_hasta: NaiveDate,
    /// Filas con el JSON de datos expandido.
    expandidos: HashSet<i64>,
    pendiente: bool,
    mensaje: Option<String>,
}

impl MovimientosView {
    pub fn marcar_pendiente(&mut self) {
        self.pendiente = true;
    }

    fn cargar(&mut self, rt: &tokio::runtime::Handle, state: &Arc<AppState>) {
        // Construye el filtro efectivo a partir de los controles de la UI
        self.filtros = repos::movimientos::FiltroMovimientos {
            desde: if self.usar_desde { Some(self.fecha_desde) } else { None },
            hasta: if self.usar_hasta { Some(self.fecha_hasta) } else { None },
            tipo: self.filtros.tipo.clone(),
            q: if self.q_texto.trim().is_empty() {
                None
            } else {
                Some(self.q_texto.clone())
            },
        };
        match rt.block_on(repos::movimientos::listar(&state.db, &self.filtros)) {
            Ok(m) => {
                self.movimientos = m;
                self.mensaje = None;
            }
            Err(e) => self.mensaje = Some(e.to_string()),
        }
    }

    pub fn mostrar(&mut self, ui: &mut egui::Ui, rt: &tokio::runtime::Handle, state: &Arc<AppState>) {
        if self.pendiente {
            self.pendiente = false;
            self.cargar(rt, state);
        }

        ui.heading("Movimientos (auditoría)");
        ui.add_space(4.0);

        // Barra de filtros
        tema::card(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new("Buscar:").color(tema::TEXTO_SEC));
            if ui
                .text_edit_singleline(&mut self.q_texto)
                .on_hover_text("Filtra por entidad o usuario")
                .changed()
            {
                self.pendiente = true;
            }
            ui.label(egui::RichText::new("Tipo:").color(tema::TEXTO_SEC));
            let actual = self.filtros.tipo.clone().unwrap_or_else(|| "(todos)".to_string());
            egui::ComboBox::from_id_source("filtro_tipo_mov")
                .selected_text(actual)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.filtros.tipo.is_none(), "(todos)")
                        .clicked()
                    {
                        self.filtros.tipo = None;
                        self.pendiente = true;
                    }
                    for t in TIPOS {
                        if ui
                            .selectable_label(self.filtros.tipo.as_deref() == Some(t), t)
                            .clicked()
                        {
                            self.filtros.tipo = Some(t.to_string());
                            self.pendiente = true;
                        }
                    }
                });
            if ui.checkbox(&mut self.usar_desde, "Desde:").changed() {
                self.pendiente = true;
            }
            if self.usar_desde
                && ui
                    .add(DatePickerButton::new(&mut self.fecha_desde).id_source("mov_fd"))
                    .changed()
            {
                self.pendiente = true;
            }
            if ui.checkbox(&mut self.usar_hasta, "Hasta:").changed() {
                self.pendiente = true;
            }
            if self.usar_hasta
                && ui
                    .add(DatePickerButton::new(&mut self.fecha_hasta).id_source("mov_fh"))
                    .changed()
            {
                self.pendiente = true;
            }
            if tema::boton_secundario(ui, "↻ Recargar").clicked() {
                self.pendiente = true;
            }
        });
        });
        if let Some(m) = &self.mensaje {
            ui.colored_label(tema::PELIGRO, m);
        }

        // Tabla de movimientos
        let mut expandidos = std::mem::take(&mut self.expandidos);
        egui::ScrollArea::both().show(ui, |ui| {
            egui::Grid::new("grid_movimientos")
                .striped(true)
                .num_columns(7)
                .show(ui, |ui| {
                    tema::cabecera_tabla(ui, "ID");
                    tema::cabecera_tabla(ui, "Tipo");
                    tema::cabecera_tabla(ui, "Entidad");
                    tema::cabecera_tabla(ui, "Id entidad");
                    tema::cabecera_tabla(ui, "Usuario");
                    tema::cabecera_tabla(ui, "Fecha y hora");
                    tema::cabecera_tabla(ui, "Datos");
                    ui.end_row();

                    for m in &self.movimientos {
                        ui.label(m.id.to_string());
                        // Color según tipo de movimiento
                        let color = match m.tipo.as_str() {
                            "creacion" => tema::EXITO,
                            "modificacion" => tema::ACENTO,
                            "cancelacion" => tema::PELIGRO,
                            _ => tema::TEXTO_SEC,
                        };
                        tema::chip_estado(ui, &m.tipo, color);
                        ui.label(&m.entidad);
                        ui.label(m.id_entidad.to_string());
                        ui.label(&m.usuario);
                        ui.label(m.timestamp.format("%d/%m/%Y %H:%M:%S").to_string());
                        // Botón para expandir/colapsar el JSON de datos
                        let abierto = expandidos.contains(&m.id);
                        let texto = if abierto { "Ocultar ▲" } else { "Ver JSON ▼" };
                        if tema::boton_tabla(ui, texto).clicked() {
                            if abierto {
                                expandidos.remove(&m.id);
                            } else {
                                expandidos.insert(m.id);
                            }
                        }
                        ui.end_row();
                        if expandidos.contains(&m.id) {
                            ui.label("");
                            ui.scope(|ui| {
                                // JSON formateado (o texto crudo si no es JSON válido)
                                let contenido = json_legible(m);
                                ui.add(
                                    egui::TextEdit::multiline(&mut contenido.as_str())
                                        .font(egui::TextStyle::Monospace)
                                        .desired_rows(6)
                                        .desired_width(700.0)
                                        .interactive(false),
                                );
                            });
                            ui.end_row();
                        }
                    }
                });
        });
        self.expandidos = expandidos;

        ui.label(format!("{} movimientos (máx. 500)", self.movimientos.len()));
    }
}

/// Formatea los JSON antes/después de un movimiento de forma legible.
fn json_legible(m: &Movimiento) -> String {
    let formatear = |s: &str| -> String {
        serde_json::from_str::<serde_json::Value>(s)
            .ok()
            .and_then(|v| serde_json::to_string_pretty(&v).ok())
            .unwrap_or_else(|| s.to_string())
    };
    let mut salida = String::new();
    if !m.datos_antes.is_empty() {
        salida.push_str(&format!("ANTES:\n{}\n\n", formatear(&m.datos_antes)));
    }
    if !m.datos_despues.is_empty() {
        salida.push_str(&format!("DESPUÉS:\n{}", formatear(&m.datos_despues)));
    }
    if salida.is_empty() {
        salida.push_str("(sin datos)");
    }
    salida
}
