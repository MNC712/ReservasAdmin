//! Vista Lista de Reservas: búsqueda, filtros y tabla con edición inline,
//! generación de comprobante PDF y cancelación.

use std::collections::HashSet;
use std::sync::Arc;

use crate::models::{EstadoReserva, Habitacion, Huesped, ReservaDetalle, ReservaInput};
use crate::repos;
use crate::repos::reservas::FiltroReservas;
use crate::services::reservas_service;
use crate::state::AppState;
use crate::ui::tema;

/// Filtros de la lista (sin filtros de fecha si no se activan las casillas).
#[derive(Default)]
struct Filtros {
    q: String,
    estado_sel: Option<EstadoReserva>,
    usar_desde: bool,
    usar_hasta: bool,
    desde: chrono::NaiveDate,
    hasta: chrono::NaiveDate,
    habitacion_sel: Option<i64>,
}

/// Panel de edición inline de una reserva.
struct Edicion {
    estado_sel: EstadoReserva,
    tarifa: String,
    notas: String,
    habitaciones_sel: HashSet<i64>,
    huespedes_sel: HashSet<i64>,
    fecha_inicio: chrono::NaiveDate,
    fecha_fin: chrono::NaiveDate,
}

/// Vista de lista de reservas.
pub struct ListaReservasView {
    reservas: Vec<ReservaDetalle>,
    habitaciones: Vec<Habitacion>,
    /// Catálogo completo de huéspedes para el panel de edición inline
    huespedes_catalogo: Vec<Huesped>,
    colores: Vec<crate::models::ColorEstado>,
    filtros: Filtros,
    editando: Option<(i64, Edicion)>,
    pendiente: bool,
    mensaje: Option<(String, bool)>,
}

impl Default for ListaReservasView {
    fn default() -> Self {
        Self {
            reservas: Vec::new(),
            habitaciones: Vec::new(),
            huespedes_catalogo: Vec::new(),
            colores: Vec::new(),
            filtros: Filtros::default(),
            editando: None,
            pendiente: true,
            mensaje: None,
        }
    }
}

impl ListaReservasView {
    pub fn marcar_pendiente(&mut self) {
        self.pendiente = true;
    }

    fn cargar(&mut self, rt: &tokio::runtime::Handle, state: &Arc<AppState>) {
        self.pendiente = false;
        let filtro = FiltroReservas {
            q: if self.filtros.q.trim().is_empty() {
                None
            } else {
                Some(self.filtros.q.clone())
            },
            estado: self.filtros.estado_sel,
            desde: if self.filtros.usar_desde {
                Some(self.filtros.desde)
            } else {
                None
            },
            hasta: if self.filtros.usar_hasta {
                Some(self.filtros.hasta)
            } else {
                None
            },
            habitacion_id: self.filtros.habitacion_sel,
        };
        let db = &state.db;
        match rt.block_on(repos::reservas::listar_detalle(db, &filtro)) {
            Ok(r) => {
                self.reservas = r;
                self.mensaje = None;
            }
            Err(e) => self.mensaje = Some((e.to_string(), true)),
        }
        self.habitaciones = rt.block_on(repos::habitaciones::listar(db)).unwrap_or_default();
        self.huespedes_catalogo = rt.block_on(repos::huespedes::listar(db)).unwrap_or_default();
        self.colores = rt.block_on(repos::config::listar_colores(db)).unwrap_or_default();
    }

    /// Color configurado para un estado (fallback gris).
    fn color_estado(&self, estado: &EstadoReserva) -> egui::Color32 {
        self.colores
            .iter()
            .find(|c| c.estado == estado.to_string())
            .map(|c| c.a_color32())
            .unwrap_or(egui::Color32::GRAY)
    }

    pub fn mostrar(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        rt: &tokio::runtime::Handle,
        state: &Arc<AppState>,
    ) {
        let _ = ctx; // reservado para ventanas futuras
        if self.pendiente {
            self.cargar(rt, state);
        }

        ui.heading("Reservas");
        ui.add_space(6.0);

        // --- Barra de búsqueda y filtros ---
        tema::card(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Buscar:").color(tema::TEXTO_SEC));
            ui.text_edit_singleline(&mut self.filtros.q);
            ui.label(egui::RichText::new("Estado:").color(tema::TEXTO_SEC));
            egui::ComboBox::from_id_source("filtro_estado")
                .selected_text(
                    self.filtros
                        .estado_sel
                        .map(|e| e.etiqueta())
                        .unwrap_or("Todos"),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.filtros.estado_sel, None, "Todos");
                    for e in EstadoReserva::TODOS {
                        ui.selectable_value(&mut self.filtros.estado_sel, Some(e), e.etiqueta());
                    }
                });
            ui.label(egui::RichText::new("Habitación:").color(tema::TEXTO_SEC));
            egui::ComboBox::from_id_source("filtro_habitacion")
                .selected_text(
                    self.filtros
                        .habitacion_sel
                        .and_then(|id| self.habitaciones.iter().find(|h| h.id == id))
                        .map(|h| h.numero.clone())
                        .unwrap_or_else(|| "Todas".to_string()),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.filtros.habitacion_sel, None, "Todas");
                    for h in &self.habitaciones {
                        ui.selectable_value(
                            &mut self.filtros.habitacion_sel,
                            Some(h.id),
                            h.numero.clone(),
                        );
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.filtros.usar_desde, "Desde:");
            if self.filtros.usar_desde {
                ui.add(egui_extras::DatePickerButton::new(&mut self.filtros.desde).id_source("fd"));
            }
            ui.checkbox(&mut self.filtros.usar_hasta, "Hasta:");
            if self.filtros.usar_hasta {
                ui.add(egui_extras::DatePickerButton::new(&mut self.filtros.hasta).id_source("fh"));
            }
            if tema::boton_primario(ui, "Aplicar filtros").clicked() {
                self.marcar_pendiente();
            }
        });
        });

        if let Some((texto, es_error)) = &self.mensaje {
            let color = if *es_error {
                tema::PELIGRO
            } else {
                tema::EXITO
            };
            ui.colored_label(color, texto);
        }

        // --- Tabla de reservas ---
        // Clonamos el catálogo para poder usarlo dentro del bucle sin conflictos de borrow
        let huespedes_catalogo = self.huespedes_catalogo.clone();
        let mut accion: Option<Accion> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("tabla_reservas")
                .num_columns(8)
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    // Cabecera
                    for titulo in [
                        "ID",
                        "Titular",
                        "Fechas",
                        "Habitaciones",
                        "Estado",
                        "Tarifa",
                        "Notas",
                        "Acciones",
                    ] {
                        tema::cabecera_tabla(ui, titulo);
                    }
                    ui.end_row();

                    for det in &self.reservas {
                        let r = &det.reserva;
                        ui.label(r.id.to_string());
                        ui.label(det.titular.nombre_completo());
                        ui.label(format!(
                            "{} → {}",
                            r.fecha_inicio.format("%d/%m/%Y"),
                            r.fecha_fin.format("%d/%m/%Y")
                        ));
                        ui.label(det.habitaciones_resumen());
                        tema::chip_estado(ui, r.estado.etiqueta(), self.color_estado(&r.estado));
                        ui.label(format!("{:.2}", r.tarifa_total));
                        ui.label(&r.notas);
                        ui.horizontal(|ui| {
                            if tema::boton_tabla(ui, "Editar").clicked() {
                                accion = Some(Accion::Editar(r.id));
                            }
                            if tema::boton_tabla(ui, "PDF").clicked() {
                                accion = Some(Accion::Pdf(r.id));
                            }
                            if r.estado != EstadoReserva::Cancelada
                                && tema::boton_tabla(ui, "Cancelar").clicked()
                            {
                                accion = Some(Accion::Cancelar(r.id));
                            }
                        });
                        ui.end_row();

                        // Panel de edición inline bajo la fila seleccionada
                        if let Some((id_ed, ed)) = &mut self.editando {
                            if *id_ed == r.id {
                                ui.label("");
                                ui.horizontal(|ui| {
                                    ui.label("Estado:");
                                    egui::ComboBox::from_id_source("ed_estado")
                                        .selected_text(ed.estado_sel.etiqueta())
                                        .show_ui(ui, |ui| {
                                            for e in EstadoReserva::TODOS {
                                                ui.selectable_value(
                                                    &mut ed.estado_sel,
                                                    e,
                                                    e.etiqueta(),
                                                );
                                            }
                                        });
                                    ui.label("Tarifa:");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut ed.tarifa)
                                            .desired_width(80.0),
                                    );
                                });
                                ui.end_row();
                                ui.label("");
                                ui.horizontal(|ui| {
                                    ui.label("Check-in:");
                                    ui.add(
                                        egui_extras::DatePickerButton::new(&mut ed.fecha_inicio)
                                            .id_source("ed_fi"),
                                    );
                                    ui.label("Check-out:");
                                    ui.add(
                                        egui_extras::DatePickerButton::new(&mut ed.fecha_fin)
                                            .id_source("ed_ff"),
                                    );
                                });
                                ui.end_row();
                                ui.label("");
                                ui.horizontal(|ui| {
                                    ui.label("Notas:");
                                    ui.text_edit_multiline(&mut ed.notas);
                                });
                                ui.end_row();
                                // Acompañantes (especialmente relevante en check-in)
                                ui.label("");
                                ui.horizontal_wrapped(|ui| {
                                    ui.label("Acompañantes:");
                                    for h in &huespedes_catalogo {
                                        let mut marcado = ed.huespedes_sel.contains(&h.id);
                                        if ui
                                            .checkbox(&mut marcado, h.nombre_completo())
                                            .clicked()
                                        {
                                            if marcado {
                                                ed.huespedes_sel.insert(h.id);
                                            } else {
                                                ed.huespedes_sel.remove(&h.id);
                                            }
                                        }
                                    }
                                });
                                ui.end_row();
                                ui.label("");
                                ui.horizontal(|ui| {
                                    if tema::boton_primario(ui, "Guardar cambios").clicked() {
                                        accion = Some(Accion::Guardar(r.id));
                                    }
                                    if tema::boton_secundario(ui, "Cerrar").clicked() {
                                        accion = Some(Accion::CerrarEdicion);
                                    }
                                });
                                ui.end_row();
                            }
                        }
                    }
                });
        });

        // --- Ejecutar acciones fuera del bucle de render ---
        match accion {
            Some(Accion::Editar(id)) => {
                if let Ok(det) = rt.block_on(repos::reservas::detalle(&state.db, id)) {
                    self.editando = Some((
                        id,
                        Edicion {
                            estado_sel: det.reserva.estado,
                            tarifa: det.reserva.tarifa_total.to_string(),
                            notas: det.reserva.notas.clone(),
                            habitaciones_sel: det.habitaciones.iter().map(|h| h.id).collect(),
                            huespedes_sel: det.huespedes.iter().map(|h| h.id).collect(),
                            fecha_inicio: det.reserva.fecha_inicio,
                            fecha_fin: det.reserva.fecha_fin,
                        },
                    ));
                }
            }
            Some(Accion::Guardar(id)) => {
                if let Some((_, ed)) = &self.editando {
                    let det_actual = self.reservas.iter().find(|d| d.reserva.id == id);
                    let cliente_id = det_actual.map(|d| d.reserva.cliente_id).unwrap_or(0);
                    let input = ReservaInput {
                        cliente_id,
                        fecha_inicio: ed.fecha_inicio,
                        fecha_fin: ed.fecha_fin,
                        estado: ed.estado_sel,
                        tarifa_total: ed.tarifa.replace(',', ".").parse().unwrap_or(0.0),
                        notas: ed.notas.clone(),
                        habitacion_ids: ed.habitaciones_sel.iter().copied().collect(),
                        huesped_ids: ed.huespedes_sel.iter().copied().collect(),
                    };
                    match rt.block_on(reservas_service::actualizar_reserva(
                        state, id, &input, "recepcion",
                    )) {
                        Ok(_) => {
                            self.mensaje = Some(("Reserva actualizada".to_string(), false));
                            self.editando = None;
                            self.marcar_pendiente();
                        }
                        Err(e) => self.mensaje = Some((e.to_string(), true)),
                    }
                }
            }
            Some(Accion::Cancelar(id)) => {
                match rt.block_on(reservas_service::cancelar_reserva(state, id, "recepcion")) {
                    Ok(_) => {
                        self.mensaje = Some(("Reserva cancelada".to_string(), false));
                        self.marcar_pendiente();
                    }
                    Err(e) => self.mensaje = Some((e.to_string(), true)),
                }
            }
            Some(Accion::Pdf(id)) => {
                match rt.block_on(crate::pdf::comprobante::generar_comprobante(
                    &state.db,
                    &state.config,
                    id,
                )) {
                    Ok(ruta) => {
                        // Abre el PDF con la aplicación predeterminada del sistema
                        if let Err(e) = open::that(&ruta) {
                            self.mensaje = Some((format!("PDF generado pero no se pudo abrir: {}", e), true));
                        } else {
                            self.mensaje = Some((
                                format!("PDF generado: {}", ruta.display()),
                                false,
                            ));
                        }
                    }
                    Err(e) => self.mensaje = Some((e.to_string(), true)),
                }
            }
            Some(Accion::CerrarEdicion) => self.editando = None,
            None => {}
        }
    }
}

/// Acciones disparadas desde la tabla.
enum Accion {
    Editar(i64),
    Guardar(i64),
    Cancelar(i64),
    Pdf(i64),
    CerrarEdicion,
}
