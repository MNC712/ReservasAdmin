//! Vista Configuración: secciones en cards para hotel, habitaciones,
//! colores, importación Excel, backups y HostelMate.

use std::path::PathBuf;
use std::sync::Arc;

use crate::excel::import::{self, ReservaImportada};
use crate::models::{ColorEstado, ConfigBackup, ExcelColumnMap, Habitacion, HotelConfig};
use crate::repos;
use crate::services::backup;
use crate::state::AppState;
use crate::ui::tema;

/// Vista de configuración con secciones agrupadas en cards.
#[derive(Default)]
pub struct ConfiguracionView {
    pendiente: bool,
    mensaje: Option<String>,
    ok: Option<String>,

    hotel: Option<HotelConfig>,

    habitaciones: Vec<Habitacion>,
    hab_form: HabForm,

    colores: Vec<ColorEstado>,

    excel_map: Vec<ExcelColumnMap>,
    excel_archivo: Option<PathBuf>,
    excel_preview: Vec<ReservaImportada>,
    excel_errores_import: Vec<String>,

    backup: Option<ConfigBackup>,
}

/// Formulario de alta/edición de habitación.
#[derive(Default)]
struct HabForm {
    id_editando: Option<i64>,
    numero: String,
    tipo: String,
    capacidad: i64,
    precio_base: f64,
    activa: bool,
}

impl ConfiguracionView {
    pub fn marcar_pendiente(&mut self) {
        self.pendiente = true;
    }

    fn cargar(&mut self, rt: &tokio::runtime::Handle, state: &Arc<AppState>) {
        let res = rt.block_on(async {
            Ok::<_, crate::error::AppError>((
                repos::config::obtener_hotel(&state.db).await?,
                repos::habitaciones::listar(&state.db).await?,
                repos::config::listar_colores(&state.db).await?,
                repos::config::listar_excel_map(&state.db).await?,
                repos::config::obtener_backup(&state.db).await?,
            ))
        });
        match res {
            Ok((hotel, habs, colores, mapa, backup)) => {
                self.hotel = Some(hotel);
                self.habitaciones = habs;
                self.colores = colores;
                self.excel_map = mapa;
                self.backup = Some(backup);
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

        ui.heading("Configuración");
        if let Some(m) = &self.mensaje {
            ui.colored_label(tema::PELIGRO, m);
        }
        if let Some(m) = &self.ok {
            ui.colored_label(tema::EXITO, m);
        }
        ui.add_space(4.0);

        egui::ScrollArea::vertical().show(ui, |ui| {
            self.seccion_hotel(ui, rt, state);
            ui.add_space(8.0);
            self.seccion_habitaciones(ui, rt, state);
            ui.add_space(8.0);
            self.seccion_colores(ui, rt, state);
            ui.add_space(8.0);
            self.seccion_excel(ui, rt, state);
            ui.add_space(8.0);
            self.seccion_backup(ui, rt, state);
            ui.add_space(8.0);
            self.seccion_hostelmate(ui, state);
        });
    }

    /// Datos generales del hotel.
    fn seccion_hotel(&mut self, ui: &mut egui::Ui, rt: &tokio::runtime::Handle, state: &Arc<AppState>) {
        tema::card(ui, |ui| {
            ui.label(tema::titulo_seccion("Datos del hotel"));
            ui.add_space(6.0);
            let Some(hotel) = &mut self.hotel else {
                ui.label("Cargando…");
                return;
            };
            egui::Grid::new("grid_hotel")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Nombre:").color(tema::TEXTO_SEC));
                    ui.text_edit_singleline(&mut hotel.nombre);
                    ui.end_row();
                    ui.label(egui::RichText::new("Dirección:").color(tema::TEXTO_SEC));
                    ui.text_edit_singleline(&mut hotel.direccion);
                    ui.end_row();
                    ui.label(egui::RichText::new("Teléfono:").color(tema::TEXTO_SEC));
                    ui.text_edit_singleline(&mut hotel.telefono);
                    ui.end_row();
                    ui.label(egui::RichText::new("Ruta del logo:").color(tema::TEXTO_SEC));
                    ui.text_edit_singleline(&mut hotel.logo_path);
                    ui.end_row();
                    ui.label(egui::RichText::new("Moneda:").color(tema::TEXTO_SEC));
                    ui.text_edit_singleline(&mut hotel.moneda);
                    ui.end_row();
                });
            ui.add_space(6.0);
            if tema::boton_primario(ui, "Guardar datos del hotel").clicked() {
                let hotel = self.hotel.clone().unwrap();
                match rt.block_on(repos::config::actualizar_hotel(&state.db, &hotel)) {
                    Ok(()) => self.ok = Some("Datos del hotel guardados.".to_string()),
                    Err(e) => self.mensaje = Some(e.to_string()),
                }
            }
        });
    }

    /// CRUD de habitaciones.
    fn seccion_habitaciones(&mut self, ui: &mut egui::Ui, rt: &tokio::runtime::Handle, state: &Arc<AppState>) {
        tema::card(ui, |ui| {
            ui.label(tema::titulo_seccion("Habitaciones"));
            ui.add_space(6.0);
            // Formulario de alta/edición
            let mut guardar = false;
            let mut cancelar_edicion = false;
            {
                let f = &mut self.hab_form;
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Número:").color(tema::TEXTO_SEC));
                    ui.add(egui::TextEdit::singleline(&mut f.numero).desired_width(60.0));
                    ui.label(egui::RichText::new("Tipo:").color(tema::TEXTO_SEC));
                    ui.add(egui::TextEdit::singleline(&mut f.tipo).desired_width(90.0));
                    ui.label(egui::RichText::new("Capacidad:").color(tema::TEXTO_SEC));
                    ui.add(egui::DragValue::new(&mut f.capacidad).clamp_range(1..=10));
                    ui.label(egui::RichText::new("Precio base:").color(tema::TEXTO_SEC));
                    ui.add(egui::DragValue::new(&mut f.precio_base).speed(1.0));
                    ui.checkbox(&mut f.activa, "Activa");
                    let texto_boton =
                        if f.id_editando.is_some() { "Actualizar" } else { "Añadir" };
                    if tema::boton_primario(ui, texto_boton).clicked() {
                        guardar = true;
                    }
                    if f.id_editando.is_some() && tema::boton_secundario(ui, "Cancelar edición").clicked() {
                        cancelar_edicion = true;
                    }
                });
            }
            if cancelar_edicion {
                self.hab_form = HabForm::default();
            }
            if guardar {
                let f = std::mem::take(&mut self.hab_form);
                {
                    let db = state.db.clone();
                    let id_edit = f.id_editando;
                    let numero = f.numero.clone();
                    let tipo = f.tipo.clone();
                    let capacidad = f.capacidad;
                    let precio = f.precio_base;
                    let activa = f.activa;
                    let resultado = rt.block_on(async move {
                        match id_edit {
                            Some(id) => {
                                repos::habitaciones::actualizar(
                                    &db, id, &numero, &tipo, capacidad, precio, activa,
                                )
                                .await
                                .map(|_| ())
                            }
                            None => repos::habitaciones::crear(
                                &db, &numero, &tipo, capacidad, precio,
                            )
                            .await
                            .map(|_| ()),
                        }
                    });
                    match resultado {
                        Ok(()) => {
                            self.ok = Some("Habitación guardada.".to_string());
                            // Recargar solo la lista de habitaciones
                            if let Ok(h) = rt.block_on(repos::habitaciones::listar(&state.db)) {
                                self.habitaciones = h;
                            }
                        }
                        Err(e) => self.mensaje = Some(e.to_string()),
                    }
                }
            }
            ui.separator();

            // Lista de habitaciones
            let mut eliminar_id = None;
            let mut editar: Option<Habitacion> = None;
            egui::Grid::new("grid_habitaciones").striped(true).num_columns(6).show(ui, |ui| {
                tema::cabecera_tabla(ui, "Número");
                tema::cabecera_tabla(ui, "Tipo");
                tema::cabecera_tabla(ui, "Capacidad");
                tema::cabecera_tabla(ui, "Precio base");
                tema::cabecera_tabla(ui, "Activa");
                tema::cabecera_tabla(ui, "Acciones");
                ui.end_row();
                for h in &self.habitaciones {
                    ui.label(&h.numero);
                    ui.label(&h.tipo);
                    ui.label(h.capacidad.to_string());
                    ui.label(format!("{:.2}", h.precio_base));
                    ui.label(if h.activa { "Sí" } else { "No" });
                    ui.horizontal(|ui| {
                        if tema::boton_tabla(ui, "Editar").clicked() {
                            editar = Some(h.clone());
                        }
                        if tema::boton_tabla(ui, "Eliminar").clicked() {
                            eliminar_id = Some(h.id);
                        }
                    });
                    ui.end_row();
                }
            });
            if let Some(h) = editar {
                self.hab_form = HabForm {
                    id_editando: Some(h.id),
                    numero: h.numero,
                    tipo: h.tipo,
                    capacidad: h.capacidad,
                    precio_base: h.precio_base,
                    activa: h.activa,
                };
            }
            if let Some(id) = eliminar_id {
                match rt.block_on(repos::habitaciones::eliminar(&state.db, id)) {
                    Ok(()) => {
                        self.ok = Some("Habitación eliminada.".to_string());
                        if let Ok(h) = rt.block_on(repos::habitaciones::listar(&state.db)) {
                            self.habitaciones = h;
                        }
                    }
                    Err(e) => self.mensaje = Some(e.to_string()),
                }
            }
        });
    }

    /// Colores por estado de reserva.
    fn seccion_colores(&mut self, ui: &mut egui::Ui, rt: &tokio::runtime::Handle, state: &Arc<AppState>) {
        tema::card(ui, |ui| {
            ui.label(tema::titulo_seccion("Colores del calendario"));
            ui.add_space(6.0);
            let mut colores_ui: Vec<egui::Color32> =
                self.colores.iter().map(|c| c.a_color32()).collect();
            let mut cambiado = false;
            ui.horizontal_wrapped(|ui| {
                for (i, c) in self.colores.iter().enumerate() {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&c.estado).color(tema::TEXTO_SEC));
                        if ui.color_edit_button_srgba(&mut colores_ui[i]).changed() {
                            cambiado = true;
                        }
                    });
                }
            });
            if cambiado {
                for (i, c) in self.colores.iter_mut().enumerate() {
                    c.desde_color32(colores_ui[i]);
                }
            }
            ui.add_space(4.0);
            if tema::boton_primario(ui, "Guardar colores").clicked() {
                let db = state.db.clone();
                let colores = self.colores.clone();
                let resultado = rt.block_on(async move {
                    for c in &colores {
                        repos::config::actualizar_color(&db, &c.estado, &c.color_hex).await?;
                    }
                    Ok::<_, crate::error::AppError>(())
                });
                match resultado {
                    Ok(()) => self.ok = Some("Colores guardados.".to_string()),
                    Err(e) => self.mensaje = Some(e.to_string()),
                }
            }
        });
    }

    /// Importación de reservas desde Excel con mapeo configurable.
    fn seccion_excel(&mut self, ui: &mut egui::Ui, rt: &tokio::runtime::Handle, state: &Arc<AppState>) {
        tema::card(ui, |ui| {
            ui.label(tema::titulo_seccion("Importación desde Excel"));
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(
                    "Mapeo de columnas (letra de columna Excel o nombre de cabecera):",
                )
                .color(tema::TEXTO_SEC),
            );
            egui::Grid::new("grid_excel_map")
                .num_columns(4)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    for par in self.excel_map.chunks_mut(2) {
                        for m in par {
                            ui.label(egui::RichText::new(format!("{}:", m.campo)).color(tema::TEXTO_SEC));
                            ui.add(
                                egui::TextEdit::singleline(&mut m.columna_excel)
                                    .desired_width(50.0),
                            );
                        }
                        ui.end_row();
                    }
                });
            ui.add_space(4.0);
            if tema::boton_primario(ui, "Guardar mapeo").clicked() {
                let db = state.db.clone();
                let mapa = self.excel_map.clone();
                let resultado = rt.block_on(async move {
                    for m in &mapa {
                        repos::config::actualizar_excel_map(&db, &m.campo, &m.columna_excel)
                            .await?;
                    }
                    Ok::<_, crate::error::AppError>(())
                });
                match resultado {
                    Ok(()) => self.ok = Some("Mapeo guardado.".to_string()),
                    Err(e) => self.mensaje = Some(e.to_string()),
                }
            }
            ui.separator();

            ui.horizontal(|ui| {
                if tema::boton_secundario(ui, "Seleccionar archivo Excel…").clicked() {
                    if let Some(ruta) = rfd::FileDialog::new()
                        .add_filter("Excel", &["xlsx", "xls"])
                        .pick_file()
                    {
                        self.excel_archivo = Some(ruta);
                    }
                }
                if let Some(ruta) = &self.excel_archivo {
                    ui.label(ruta.display().to_string());
                }
            });
            if self.excel_archivo.is_some() && tema::boton_secundario(ui, "Vista previa").clicked() {
                let ruta = self.excel_archivo.clone().unwrap();
                let db = state.db.clone();
                let ruta_txt = ruta.to_string_lossy().to_string();
                match rt.block_on(async move { import::importar_reservas(&db, &ruta_txt).await })
                {
                    Ok(items) => {
                        self.ok = Some(format!("Vista previa: {} filas leídas.", items.len()));
                        self.excel_preview = items;
                        self.excel_errores_import.clear();
                    }
                    Err(e) => self.mensaje = Some(e.to_string()),
                }
            }

            // Tabla de vista previa
            if !self.excel_preview.is_empty() {
                ui.separator();
                ui.label(format!("Vista previa ({} filas):", self.excel_preview.len()));
                egui::Grid::new("grid_preview_excel").striped(true).num_columns(7).show(ui, |ui| {
                    tema::cabecera_tabla(ui, "Titular");
                    tema::cabecera_tabla(ui, "Documento");
                    tema::cabecera_tabla(ui, "Entrada");
                    tema::cabecera_tabla(ui, "Salida");
                    tema::cabecera_tabla(ui, "Habitación");
                    tema::cabecera_tabla(ui, "Tarifa");
                    tema::cabecera_tabla(ui, "Estado");
                    ui.end_row();
                    for item in &self.excel_preview {
                        ui.label(&item.titular_nombre);
                        ui.label(&item.titular_documento);
                        ui.label(
                            item.fecha_inicio
                                .map(|f| f.format("%d/%m/%Y").to_string())
                                .unwrap_or_default(),
                        );
                        ui.label(
                            item.fecha_fin
                                .map(|f| f.format("%d/%m/%Y").to_string())
                                .unwrap_or_default(),
                        );
                        ui.label(&item.habitacion);
                        ui.label(format!("{:.2}", item.tarifa));
                        if item.errores.is_empty() {
                            ui.colored_label(tema::EXITO, "OK");
                        } else {
                            ui.colored_label(tema::PELIGRO, item.errores.join("; "));
                        }
                        ui.end_row();
                    }
                });
                let validas = self
                    .excel_preview
                    .iter()
                    .filter(|i| i.errores.is_empty())
                    .count();
                if tema::boton_primario(ui, &format!("Importar {} reservas válidas", validas))
                    .clicked()
                {
                    let items = self.excel_preview.clone();
                    let st = state.clone();
                    match rt
                        .block_on(async move { import::insertar_importadas(&st, items).await })
                    {
                        Ok((n, errores)) => {
                            self.ok = Some(format!("{} reservas importadas.", n));
                            self.excel_errores_import = errores;
                            self.excel_preview.clear();
                            self.excel_archivo = None;
                        }
                        Err(e) => self.mensaje = Some(e.to_string()),
                    }
                }
                for err in &self.excel_errores_import {
                    ui.colored_label(tema::AVISO, err);
                }
            }
        });
    }

    /// Copias de seguridad de la base de datos.
    fn seccion_backup(&mut self, ui: &mut egui::Ui, rt: &tokio::runtime::Handle, state: &Arc<AppState>) {
        tema::card(ui, |ui| {
            ui.label(tema::titulo_seccion("Copias de seguridad"));
            ui.add_space(6.0);
            let Some(cfg) = &mut self.backup else {
                ui.label("Cargando…");
                return;
            };
            egui::Grid::new("grid_backup")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("Carpeta destino:").color(tema::TEXTO_SEC));
                    ui.text_edit_singleline(&mut cfg.carpeta);
                    ui.end_row();
                    ui.label(egui::RichText::new("Intervalo (minutos):").color(tema::TEXTO_SEC));
                    ui.add(egui::DragValue::new(&mut cfg.intervalo_min).range(1..=10080));
                    ui.end_row();
                    ui.label(egui::RichText::new("Retención (días):").color(tema::TEXTO_SEC));
                    ui.add(egui::DragValue::new(&mut cfg.retencion_dias).range(1..=365));
                    ui.end_row();
                });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if tema::boton_primario(ui, "Guardar configuración").clicked() {
                    let cfg = self.backup.clone().unwrap();
                    match rt.block_on(repos::config::actualizar_backup(&state.db, &cfg)) {
                        Ok(()) => {
                            self.ok = Some("Configuración de backup guardada.".to_string())
                        }
                        Err(e) => self.mensaje = Some(e.to_string()),
                    }
                }
                if tema::boton_secundario(ui, "▶ Ejecutar backup ahora").clicked() {
                    let st = state.clone();
                    match rt.block_on(async move { backup::ejecutar_backup_ahora(&st).await }) {
                        Ok(ruta) => {
                            self.ok = Some(format!("Backup creado en: {}", ruta.display()))
                        }
                        Err(e) => self.mensaje = Some(e.to_string()),
                    }
                }
            });
        });
    }

    /// Integración con HostelMate (solo lectura del estado).
    fn seccion_hostelmate(&mut self, ui: &mut egui::Ui, state: &Arc<AppState>) {
        tema::card(ui, |ui| {
            ui.label(tema::titulo_seccion("Integración HostelMate"));
            ui.add_space(6.0);
            egui::Grid::new("grid_hostelmate")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .show(ui, |ui| {
                    ui.label(egui::RichText::new("URL base:").color(tema::TEXTO_SEC));
                    ui.label(&state.config.hostelmate_base_url);
                    ui.end_row();
                    ui.label(egui::RichText::new("API key:").color(tema::TEXTO_SEC));
                    // Se muestran solo los primeros caracteres; nunca la clave completa
                    let clave = &state.config.hostelmate_api_key;
                    let enmascarada = if clave.is_empty() {
                        "(no configurada)".to_string()
                    } else {
                        let prefijo: String = clave.chars().take(4).collect();
                        format!("{}••••••••", prefijo)
                    };
                    ui.label(enmascarada);
                    ui.end_row();
                    ui.label(egui::RichText::new("Estado:").color(tema::TEXTO_SEC));
                    if state.hostelmate.habilitado() {
                        ui.label(
                            egui::RichText::new("Habilitado")
                                .strong()
                                .color(tema::EXITO),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("Deshabilitado (sin API key)")
                                .color(tema::AVISO),
                        );
                    }
                    ui.end_row();
                });
        });
    }
}
