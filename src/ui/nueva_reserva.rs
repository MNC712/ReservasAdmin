//! Vista Nueva Reserva: formulario de creación con titular, fechas,
//! habitaciones (multi-selección), huéspedes, tarifa y notas.

use std::collections::HashSet;
use std::sync::Arc;

use chrono::Local;

use crate::models::{Cliente, EstadoReserva, Habitacion, Huesped, ReservaInput};
use crate::repos;
use crate::services::reservas_service;
use crate::state::AppState;
use crate::ui::tema;

/// Formulario de creación de reservas.
pub struct NuevaReservaView {
    // Catálogos cargados de BD
    clientes: Vec<Cliente>,
    habitaciones: Vec<Habitacion>,
    huespedes: Vec<Huesped>,
    pendiente: bool,

    // Campos del formulario
    cliente_sel: Option<i64>,
    fecha_inicio: chrono::NaiveDate,
    fecha_fin: chrono::NaiveDate,
    estado_sel: EstadoReserva,
    habitaciones_sel: HashSet<i64>,
    huespedes_sel: HashSet<i64>,
    tarifa: String,
    notas: String,

    // Formulario inline de nuevo titular
    nuevo_cliente_visible: bool,
    nc_nombre: String,
    nc_apellidos: String,
    nc_documento: String,
    nc_telefono: String,

    // Formulario inline de nuevo acompañante
    nuevo_huesped_visible: bool,
    nh_nombre: String,
    nh_apellidos: String,
    nh_documento: String,

    // Resultado de la última operación (texto, es_error)
    mensaje: Option<(String, bool)>,
}

impl Default for NuevaReservaView {
    fn default() -> Self {
        let hoy = Local::now().date_naive();
        Self {
            clientes: Vec::new(),
            habitaciones: Vec::new(),
            huespedes: Vec::new(),
            pendiente: true,
            cliente_sel: None,
            fecha_inicio: hoy,
            fecha_fin: hoy + chrono::Duration::days(1),
            estado_sel: EstadoReserva::Confirmada,
            habitaciones_sel: HashSet::new(),
            huespedes_sel: HashSet::new(),
            tarifa: String::new(),
            notas: String::new(),
            nuevo_cliente_visible: false,
            nc_nombre: String::new(),
            nc_apellidos: String::new(),
            nc_documento: String::new(),
            nc_telefono: String::new(),
            nuevo_huesped_visible: false,
            nh_nombre: String::new(),
            nh_apellidos: String::new(),
            nh_documento: String::new(),
            mensaje: None,
        }
    }
}

impl NuevaReservaView {
    pub fn marcar_pendiente(&mut self) {
        self.pendiente = true;
    }

    /// Carga/recarga los catálogos desde la BD.
    fn cargar(&mut self, rt: &tokio::runtime::Handle, state: &Arc<AppState>) {
        self.pendiente = false;
        let db = &state.db;
        self.clientes = rt.block_on(repos::clientes::listar(db)).unwrap_or_default();
        self.habitaciones = rt
            .block_on(repos::habitaciones::listar_activas(db))
            .unwrap_or_default();
        self.huespedes = rt.block_on(repos::huespedes::listar(db)).unwrap_or_default();
    }

    pub fn mostrar(&mut self, ui: &mut egui::Ui, rt: &tokio::runtime::Handle, state: &Arc<AppState>) {
        if self.pendiente {
            self.cargar(rt, state);
        }

        ui.heading("Nueva reserva");
        ui.add_space(8.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            // --- Titular ---
            tema::card(ui, |ui| {
                ui.label(tema::titulo_seccion("Titular"));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let texto_sel = self
                        .cliente_sel
                        .and_then(|id| self.clientes.iter().find(|c| c.id == id))
                        .map(|c| format!("{} ({})", c.nombre_completo(), c.documento))
                        .unwrap_or_else(|| "Seleccionar...".to_string());
                    egui::ComboBox::from_id_source("combo_cliente")
                        .selected_text(texto_sel)
                        .show_ui(ui, |ui| {
                            for c in &self.clientes {
                                ui.selectable_value(
                                    &mut self.cliente_sel,
                                    Some(c.id),
                                    format!("{} ({})", c.nombre_completo(), c.documento),
                                );
                            }
                        });
                    if tema::boton_secundario(ui, "＋ Nuevo").clicked() {
                        self.nuevo_cliente_visible = !self.nuevo_cliente_visible;
                    }
                });

                // Alta rápida de titular
                if self.nuevo_cliente_visible {
                    ui.add_space(4.0);
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.label(egui::RichText::new("Nuevo cliente:").color(tema::TEXTO_SEC));
                        ui.horizontal(|ui| {
                            ui.label("Nombre:");
                            ui.text_edit_singleline(&mut self.nc_nombre);
                            ui.label("Apellidos:");
                            ui.text_edit_singleline(&mut self.nc_apellidos);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Documento:");
                            ui.text_edit_singleline(&mut self.nc_documento);
                            ui.label("Teléfono:");
                            ui.text_edit_singleline(&mut self.nc_telefono);
                            if tema::boton_primario(ui, "Crear cliente").clicked()
                                && !self.nc_nombre.trim().is_empty()
                            {
                                let resultado = rt.block_on(repos::clientes::crear(
                                    &state.db,
                                    &self.nc_nombre,
                                    &self.nc_apellidos,
                                    &self.nc_documento,
                                    "",
                                    &self.nc_telefono,
                                ));
                                match resultado {
                                    Ok(c) => {
                                        self.cliente_sel = Some(c.id);
                                        self.marcar_pendiente();
                                        self.nuevo_cliente_visible = false;
                                        self.mensaje = Some(("Cliente creado".to_string(), false));
                                    }
                                    Err(e) => self.mensaje = Some((e.to_string(), true)),
                                }
                            }
                        });
                    });
                }
            });

            ui.add_space(8.0);

            // --- Estancia: fechas, estado y habitaciones ---
            tema::card(ui, |ui| {
                ui.label(tema::titulo_seccion("Estancia"));
                ui.add_space(6.0);
                egui::Grid::new("grid_estancia")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Check-in:").color(tema::TEXTO_SEC));
                        ui.horizontal(|ui| {
                            ui.add(
                                egui_extras::DatePickerButton::new(&mut self.fecha_inicio)
                                    .id_source("fi"),
                            );
                            ui.label(egui::RichText::new("Check-out:").color(tema::TEXTO_SEC));
                            ui.add(
                                egui_extras::DatePickerButton::new(&mut self.fecha_fin)
                                    .id_source("ff"),
                            );
                        });
                        ui.end_row();
                        ui.label(egui::RichText::new("Estado:").color(tema::TEXTO_SEC));
                        egui::ComboBox::from_id_source("combo_estado")
                            .selected_text(self.estado_sel.etiqueta())
                            .show_ui(ui, |ui| {
                                for e in EstadoReserva::TODOS {
                                    ui.selectable_value(&mut self.estado_sel, e, e.etiqueta());
                                }
                            });
                        ui.end_row();
                    });

                ui.add_space(6.0);
                ui.label(egui::RichText::new("Habitaciones:").color(tema::TEXTO_SEC));
                ui.horizontal_wrapped(|ui| {
                    for hab in &self.habitaciones {
                        let mut marcada = self.habitaciones_sel.contains(&hab.id);
                        if ui
                            .checkbox(&mut marcada, format!("{} ({})", hab.numero, hab.tipo))
                            .clicked()
                        {
                            if marcada {
                                self.habitaciones_sel.insert(hab.id);
                            } else {
                                self.habitaciones_sel.remove(&hab.id);
                            }
                        }
                    }
                });
            });

            ui.add_space(8.0);

            // --- Huéspedes / acompañantes ---
            tema::card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tema::titulo_seccion("Huéspedes / acompañantes"));
                    if tema::boton_secundario(ui, "＋ Nuevo huésped").clicked() {
                        self.nuevo_huesped_visible = !self.nuevo_huesped_visible;
                    }
                });
                ui.add_space(4.0);
                if self.nuevo_huesped_visible {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Nombre:");
                            ui.text_edit_singleline(&mut self.nh_nombre);
                            ui.label("Apellidos:");
                            ui.text_edit_singleline(&mut self.nh_apellidos);
                            ui.label("Documento:");
                            ui.text_edit_singleline(&mut self.nh_documento);
                            if tema::boton_primario(ui, "Crear huésped").clicked()
                                && !self.nh_nombre.trim().is_empty()
                            {
                                let resultado = rt.block_on(repos::huespedes::crear(
                                    &state.db,
                                    &self.nh_nombre,
                                    &self.nh_apellidos,
                                    &self.nh_documento,
                                    "",
                                    "",
                                ));
                                match resultado {
                                    Ok(h) => {
                                        self.huespedes_sel.insert(h.id);
                                        self.marcar_pendiente();
                                        self.nuevo_huesped_visible = false;
                                        self.mensaje =
                                            Some(("Huésped creado".to_string(), false));
                                    }
                                    Err(e) => self.mensaje = Some((e.to_string(), true)),
                                }
                            }
                        });
                    });
                }
                ui.horizontal_wrapped(|ui| {
                    for h in &self.huespedes {
                        let mut marcado = self.huespedes_sel.contains(&h.id);
                        if ui.checkbox(&mut marcado, h.nombre_completo()).clicked() {
                            if marcado {
                                self.huespedes_sel.insert(h.id);
                            } else {
                                self.huespedes_sel.remove(&h.id);
                            }
                        }
                    }
                });
            });

            ui.add_space(8.0);

            // --- Tarifa y notas ---
            tema::card(ui, |ui| {
                ui.label(tema::titulo_seccion("Tarifa y notas"));
                ui.add_space(6.0);
                egui::Grid::new("grid_tarifa")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Tarifa total:").color(tema::TEXTO_SEC));
                        ui.add(egui::TextEdit::singleline(&mut self.tarifa).desired_width(100.0));
                        ui.end_row();
                        ui.label(egui::RichText::new("Notas:").color(tema::TEXTO_SEC));
                        ui.text_edit_multiline(&mut self.notas);
                        ui.end_row();
                    });
            });

            ui.add_space(10.0);

            // --- Guardar ---
            if tema::boton_primario(ui, "Guardar reserva").clicked() {
                let input = ReservaInput {
                    cliente_id: self.cliente_sel.unwrap_or(0),
                    fecha_inicio: self.fecha_inicio,
                    fecha_fin: self.fecha_fin,
                    estado: self.estado_sel,
                    tarifa_total: self.tarifa.replace(',', ".").parse().unwrap_or(0.0),
                    notas: self.notas.clone(),
                    habitacion_ids: self.habitaciones_sel.iter().copied().collect(),
                    huesped_ids: self.huespedes_sel.iter().copied().collect(),
                };
                if input.cliente_id == 0 {
                    self.mensaje = Some(("Seleccione un titular".to_string(), true));
                } else {
                    match rt.block_on(reservas_service::crear_reserva(state, &input, "recepcion")) {
                        Ok(det) => {
                            let msg = format!("Reserva #{} creada correctamente", det.reserva.id);
                            // Limpia el formulario conservando el mensaje de éxito
                            *self = NuevaReservaView::default();
                            self.mensaje = Some((msg, false));
                        }
                        Err(e) => self.mensaje = Some((e.to_string(), true)),
                    }
                }
            }

            // --- Mensajes de éxito/error ---
            if let Some((texto, es_error)) = &self.mensaje {
                let color = if *es_error {
                    tema::PELIGRO
                } else {
                    tema::EXITO
                };
                ui.colored_label(color, texto);
            }
        });
    }
}
