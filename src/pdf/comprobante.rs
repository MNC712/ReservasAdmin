//! Generación de comprobantes de reserva en PDF con genpdf.
//!
//! Se usan las fuentes DejaVu empaquetadas en `assets/fonts/` (con fallback
//! a las fuentes Arial de Windows si faltaran). El documento tiene cabecera
//! decorada, secciones con tablas etiqueta/valor, lista de huéspedes y total
//! destacado.

use std::path::PathBuf;

use chrono::Local;
use genpdf::{elements, fonts, style, Alignment, Document, Element, Position, RenderResult, SimplePageDecorator, Size};
use sqlx::SqlitePool;

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::repos;

/// Acento azul de la aplicación (#2563EB)
const ACENTO: style::Color = style::Color::Rgb(37, 99, 235);
/// Gris de texto secundario (#6B7280)
const GRIS: style::Color = style::Color::Rgb(107, 114, 128);
/// Texto principal (#1F2933)
const TEXTO: style::Color = style::Color::Rgb(31, 41, 51);

/// Carga la familia de fuentes DejaVu de `assets/fonts/`
/// (fallback: Arial de Windows si el archivo no existe).
fn cargar_fuente() -> AppResult<fonts::FontFamily<fonts::FontData>> {
    let dir = std::path::Path::new("assets/fonts");
    let regular = fonts::FontData::load(dir.join("DejaVuSans.ttf"), None)
        .or_else(|_| fonts::FontData::load(r"C:\Windows\Fonts\arial.ttf", None))
        .map_err(|e| AppError::Pdf(format!("no se pudo cargar la fuente: {}", e)))?;
    let cargar_o = |nombres: [&str; 2]| {
        fonts::FontData::load(dir.join(nombres[0]), None)
            .or_else(|_| fonts::FontData::load(nombres[1], None))
            .unwrap_or_else(|_| regular.clone())
    };
    Ok(fonts::FontFamily {
        bold: cargar_o(["DejaVuSans-Bold.ttf", r"C:\Windows\Fonts\arialbd.ttf"]),
        italic: cargar_o(["DejaVuSans-Oblique.ttf", r"C:\Windows\Fonts\ariali.ttf"]),
        bold_italic: cargar_o(["DejaVuSans-BoldOblique.ttf", r"C:\Windows\Fonts\arialbi.ttf"]),
        regular,
    })
}

/// Elemento mínimo: línea horizontal decorativa del ancho disponible.
/// genpdf 0.2 solo dibuja líneas finas; se hacen varias pasadas para
/// simular un grosor de ~1.5 mm.
struct LineaHorizontal {
    color: style::Color,
    altura_mm: f64,
}

impl Element for LineaHorizontal {
    fn render(
        &mut self,
        _context: &genpdf::Context,
        area: genpdf::render::Area<'_>,
        style: style::Style,
    ) -> Result<RenderResult, genpdf::error::Error> {
        let mut estilo = style;
        estilo.set_color(self.color);
        let ancho = area.size().width;
        let pasadas = (self.altura_mm / 0.35).ceil().max(1.0) as usize;
        for i in 0..pasadas {
            let y = self.altura_mm / 2.0 - 0.175 + i as f64 * 0.35;
            area.draw_line(vec![Position::new(0, y), Position::new(ancho, y)], estilo);
        }
        Ok(RenderResult {
            size: Size::new(ancho, self.altura_mm),
            has_more: false,
        })
    }
}

/// Título de sección (negrita, color acento).
fn titulo_seccion(texto: &str) -> elements::StyledElement<elements::Paragraph> {
    elements::Paragraph::new(texto)
        .styled(style::Style::new().bold().with_font_size(12).with_color(ACENTO))
}

/// Tabla de 2 columnas etiqueta (gris 9pt) / valor (10pt) con algo de aire.
fn tabla_datos(filas: Vec<(String, String)>) -> elements::TableLayout {
    let mut tabla = elements::TableLayout::new(vec![1, 3]);
    for (etiqueta, valor) in filas {
        let fila = tabla
            .row()
            .element(
                elements::Paragraph::new(etiqueta)
                    .styled(style::Style::new().with_font_size(9).with_color(GRIS))
                    .padded(1.0),
            )
            .element(
                elements::Paragraph::new(valor)
                    .styled(style::Style::new().with_font_size(10).with_color(TEXTO))
                    .padded(1.0),
            )
            .push();
        // Las filas siempre tienen 2 celdas: no puede fallar
        let _ = fila;
    }
    tabla
}

/// Genera el comprobante PDF de una reserva en `PDF_OUTPUT_DIR`
/// (por defecto `comprobantes/`). Devuelve la ruta del archivo creado.
pub async fn generar_comprobante(
    db: &SqlitePool,
    config: &AppConfig,
    reserva_id: i64,
) -> AppResult<PathBuf> {
    let det = repos::reservas::detalle(db, reserva_id).await?;
    let hotel = repos::config::obtener_hotel(db).await.unwrap_or(
        crate::models::HotelConfig {
            id: 1,
            nombre: String::new(),
            direccion: String::new(),
            telefono: String::new(),
            logo_path: String::new(),
            moneda: "EUR".to_string(),
        },
    );

    let fuente = cargar_fuente()?;
    let mut doc = Document::new(fuente);
    doc.set_title(format!("Comprobante de reserva #{}", det.reserva.id));
    doc.set_paper_size(genpdf::PaperSize::A4);

    // Márgenes de página de 18 mm
    let mut decorador = SimplePageDecorator::new();
    decorador.set_margins(18);
    doc.set_page_decorator(decorador);

    // --- 1. Cabecera: nombre del hotel + contacto + línea decorativa ---
    doc.push(
        elements::Paragraph::new(&hotel.nombre)
            .styled(style::Style::new().bold().with_font_size(19).with_color(ACENTO)),
    );
    let mut contacto = String::new();
    if !hotel.direccion.is_empty() {
        contacto.push_str(&hotel.direccion);
    }
    if !hotel.telefono.is_empty() {
        if !contacto.is_empty() {
            contacto.push_str(" · ");
        }
        contacto.push_str(&format!("Tel: {}", hotel.telefono));
    }
    if !contacto.is_empty() {
        doc.push(
            elements::Paragraph::new(contacto)
                .styled(style::Style::new().with_font_size(9).with_color(GRIS)),
        );
    }
    doc.push(LineaHorizontal { color: ACENTO, altura_mm: 1.5 });
    doc.push(elements::Break::new(1.0));

    // --- 2. Título del documento ---
    doc.push(
        elements::Paragraph::new("COMPROBANTE DE RESERVA")
            .aligned(Alignment::Center)
            .styled(style::Style::new().bold().with_font_size(14).with_color(TEXTO)),
    );
    doc.push(
        elements::Paragraph::new(format!(
            "Nº RES-{} · Emitido: {}",
            det.reserva.id,
            Local::now().format("%d/%m/%Y")
        ))
        .aligned(Alignment::Center)
        .styled(style::Style::new().with_font_size(9).with_color(GRIS)),
    );
    doc.push(elements::Break::new(1.2));

    // --- 3. Titular ---
    doc.push(titulo_seccion("Titular"));
    let mut filas_titular = vec![("Nombre".to_string(), det.titular.nombre_completo())];
    if !det.titular.documento.is_empty() {
        filas_titular.push(("Documento".to_string(), det.titular.documento.clone()));
    }
    if !det.titular.email.is_empty() {
        filas_titular.push(("Email".to_string(), det.titular.email.clone()));
    }
    if !det.titular.telefono.is_empty() {
        filas_titular.push(("Teléfono".to_string(), det.titular.telefono.clone()));
    }
    doc.push(tabla_datos(filas_titular));
    doc.push(elements::Break::new(1.0));

    // --- 4. Estancia ---
    doc.push(titulo_seccion("Estancia"));
    let noches = (det.reserva.fecha_fin - det.reserva.fecha_inicio).num_days();
    doc.push(tabla_datos(vec![
        (
            "Entrada".to_string(),
            det.reserva.fecha_inicio.format("%d/%m/%Y").to_string(),
        ),
        (
            "Salida".to_string(),
            det.reserva.fecha_fin.format("%d/%m/%Y").to_string(),
        ),
        ("Noches".to_string(), noches.to_string()),
        ("Estado".to_string(), det.reserva.estado.etiqueta().to_string()),
        ("Habitaciones".to_string(), det.habitaciones_resumen()),
    ]));
    doc.push(elements::Break::new(1.0));

    // --- 5. Huéspedes / acompañantes ---
    doc.push(titulo_seccion("Huéspedes / Acompañantes"));
    if det.huespedes.is_empty() {
        doc.push(
            elements::Paragraph::new("Sin acompañantes registrados")
                .styled(style::Style::new().italic().with_font_size(9).with_color(GRIS)),
        );
    } else {
        let mut lista = elements::UnorderedList::new();
        for h in &det.huespedes {
            let mut texto = h.nombre_completo();
            if !h.documento.is_empty() {
                texto.push_str(&format!(" — {}", h.documento));
            }
            lista.push(
                elements::Paragraph::new(texto)
                    .styled(style::Style::new().with_font_size(10).with_color(TEXTO)),
            );
        }
        doc.push(lista);
    }
    doc.push(elements::Break::new(1.2));

    // --- 6. Total destacado ---
    let mut tabla_total = elements::TableLayout::new(vec![1, 1]);
    let _ = tabla_total
        .row()
        .element(
            elements::Paragraph::new("TOTAL")
                .styled(style::Style::new().bold().with_font_size(13).with_color(TEXTO))
                .padded(2.0),
        )
        .element(
            elements::Paragraph::new(format!("{:.2} {}", det.reserva.tarifa_total, hotel.moneda))
                .aligned(Alignment::Right)
                .styled(style::Style::new().bold().with_font_size(13).with_color(ACENTO))
                .padded(2.0),
        )
        .push();
    // Caja con borde acentuado (genpdf 0.2 no tiene relleno de fondo)
    doc.push(
        elements::StyledElement::new(
            elements::FramedElement::new(tabla_total),
            style::Style::new().with_color(ACENTO),
        )
        .padded(1.0),
    );

    // --- 7. Notas ---
    if !det.reserva.notas.is_empty() {
        doc.push(elements::Break::new(0.8));
        doc.push(
            elements::Paragraph::new(format!("Notas: {}", det.reserva.notas))
                .styled(style::Style::new().with_font_size(9).with_color(GRIS)),
        );
    }

    // --- 8. Pie: línea sutil + agradecimiento ---
    doc.push(elements::Break::new(1.5));
    doc.push(LineaHorizontal { color: GRIS, altura_mm: 0.4 });
    doc.push(elements::Break::new(0.5));
    doc.push(
        elements::Paragraph::new(format!("Gracias por su reserva — {}", hotel.nombre))
            .aligned(Alignment::Center)
            .styled(style::Style::new().with_font_size(9).with_color(GRIS)),
    );

    // Guardar en la carpeta configurada
    std::fs::create_dir_all(&config.pdf_output_dir)?;
    let ruta = PathBuf::from(&config.pdf_output_dir)
        .join(format!("comprobante_reserva_{}.pdf", det.reserva.id));
    doc.render_to_file(&ruta)
        .map_err(|e| AppError::Pdf(e.to_string()))?;
    Ok(ruta)
}
