//! Tema visual global de la aplicación: paleta minimalista clara,
//! tipografía, espaciado y helpers de estilo reutilizables
//! (cards, chips de estado y botones uniformes).
//!
//! El renderer es wgpu por software: el estilo es plano (flat),
//! sin sombras ni efectos caros.

use egui::{Color32, Context, FontFamily, FontId, Margin, RichText, Rounding, Stroke, TextStyle, Visuals};

// --- Paleta minimalista clara ---
/// Fondo general de la aplicación (#F7F7F5)
pub const BG_APP: Color32 = Color32::from_rgb(0xF7, 0xF7, 0xF5);
/// Fondo de paneles y tarjetas (#FFFFFF)
pub const PANEL: Color32 = Color32::WHITE;
/// Borde sutil de tarjetas y widgets (#E5E7EB)
pub const BORDE: Color32 = Color32::from_rgb(0xE5, 0xE7, 0xEB);
/// Texto principal (#1F2933)
pub const TEXTO: Color32 = Color32::from_rgb(0x1F, 0x29, 0x33);
/// Texto secundario: etiquetas, cabeceras de tabla (#6B7280)
pub const TEXTO_SEC: Color32 = Color32::from_rgb(0x6B, 0x72, 0x80);
/// Acento único, azul (#2563EB)
pub const ACENTO: Color32 = Color32::from_rgb(0x25, 0x63, 0xEB);
/// Acento suave para fondos de hover/selección (#EFF4FF)
pub const ACENTO_SUAVE: Color32 = Color32::from_rgb(0xEF, 0xF4, 0xFF);
/// Éxito, verde (#059669)
pub const EXITO: Color32 = Color32::from_rgb(0x05, 0x96, 0x69);
/// Peligro, rojo (#DC2626)
pub const PELIGRO: Color32 = Color32::from_rgb(0xDC, 0x26, 0x26);
/// Aviso, ámbar (#D97706)
pub const AVISO: Color32 = Color32::from_rgb(0xD9, 0x77, 0x06);
/// Gris muy claro: cabeceras de tabla y celdas (#F3F4F6)
pub const FAINT: Color32 = Color32::from_rgb(0xF3, 0xF4, 0xF6);

/// TextStyle para títulos de sección dentro de las cards (17px)
pub const TITULO_SECCION: &str = "TituloSeccion";
/// Tamaño del número grande de las tarjetas KPI
pub const TAM_KPI: f32 = 27.0;

/// Aplica el tema minimalista claro a todo el contexto egui.
/// Debe llamarse una sola vez al arrancar la aplicación.
pub fn aplicar_tema(ctx: &Context) {
    let mut style = (*ctx.style()).clone();

    // Tipografía (egui no tiene peso "semibold": se simula con strong())
    style.text_styles = [
        (TextStyle::Heading, FontId::new(22.0, FontFamily::Proportional)),
        (
            TextStyle::Name(TITULO_SECCION.into()),
            FontId::new(17.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
    ]
    .into();

    // Espaciado uniforme
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 7.0);
    style.spacing.indent = 16.0;

    // Visuals claros y planos
    let mut v = Visuals::light();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = BG_APP;
    v.faint_bg_color = FAINT;
    v.window_rounding = Rounding::same(6.0);
    v.menu_rounding = Rounding::same(6.0);
    v.selection.bg_fill = ACENTO.linear_multiply(0.35);

    v.widgets.noninteractive.rounding = Rounding::same(5.0);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDE);

    v.widgets.inactive.rounding = Rounding::same(5.0);
    v.widgets.inactive.weak_bg_fill = Color32::WHITE;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDE);

    v.widgets.hovered.rounding = Rounding::same(5.0);
    v.widgets.hovered.weak_bg_fill = ACENTO_SUAVE;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACENTO);

    v.widgets.active.rounding = Rounding::same(5.0);
    v.widgets.active.weak_bg_fill = ACENTO;
    v.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACENTO);

    v.widgets.open.rounding = Rounding::same(5.0);

    style.visuals = v;
    ctx.set_style(style);
}

/// Card: panel blanco con borde sutil, esquinas de 8px y margen interno uniforme.
pub fn card(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::InnerResponse<()> {
    egui::Frame::none()
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, BORDE))
        .rounding(Rounding::same(8.0))
        .inner_margin(Margin::same(16.0))
        .show(ui, add_contents)
}

/// Título de sección (17px, semibold simulado con strong()).
pub fn titulo_seccion(texto: &str) -> RichText {
    RichText::new(texto)
        .text_style(TextStyle::Name(TITULO_SECCION.into()))
        .strong()
}

/// Pastilla de estado: fondo del color con alpha suave, texto del color
/// oscurecido, esquinas muy redondeadas y sin borde.
pub fn chip_estado(ui: &mut egui::Ui, texto: &str, color: Color32) -> egui::Response {
    egui::Frame::none()
        .fill(color.linear_multiply(0.14))
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin::symmetric(10.0, 3.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(texto)
                    .size(12.0)
                    .strong()
                    .color(color.gamma_multiply(0.75)),
            );
        })
        .response
}

/// Botón de acción primario (fondo acento, texto blanco), tamaño mínimo uniforme.
pub fn boton_primario(ui: &mut egui::Ui, texto: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(texto).color(Color32::WHITE))
            .fill(ACENTO)
            .min_size(egui::vec2(120.0, 30.0)),
    )
}

/// Botón de acción secundario (fondo blanco, borde sutil), tamaño mínimo uniforme.
pub fn boton_secundario(ui: &mut egui::Ui, texto: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(texto).color(TEXTO))
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0_f32, BORDE))
            .min_size(egui::vec2(120.0, 30.0)),
    )
}

/// Botón compacto para acciones dentro de tablas (mismo tamaño entre ellos).
pub fn boton_tabla(ui: &mut egui::Ui, texto: &str) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(texto).size(12.0).color(TEXTO))
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0_f32, BORDE))
            .min_size(egui::vec2(64.0, 22.0)),
    )
}

/// Cabecera de tabla: texto pequeño, negrita y gris secundario.
pub fn cabecera_tabla(ui: &mut egui::Ui, texto: &str) {
    ui.label(RichText::new(texto).small().strong().color(TEXTO_SEC));
}
