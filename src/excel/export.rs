//! Export de reservas a Excel con rust_xlsxwriter.
//! Se reescribe el archivo COMPLETO tras cada mutación (estrategia simple
//! y segura para volúmenes de hotel pequeño/mediano).

use rust_xlsxwriter::{Format, Workbook};
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::repos::reservas::{self, FiltroReservas};

/// Columnas de la hoja "Reservas".
const CABECERAS: [&str; 10] = [
    "ID",
    "Titular",
    "Documento",
    "Fecha inicio",
    "Fecha fin",
    "Habitaciones",
    "Huéspedes",
    "Estado",
    "Tarifa",
    "Notas",
];

/// Reexporta todas las reservas al archivo `ruta` (hoja "Reservas").
pub async fn exportar_reservas(db: &SqlitePool, ruta: &str) -> AppResult<()> {
    let detalles = reservas::listar_detalle(db, &FiltroReservas::default()).await?;

    let mut libro = Workbook::new();
    let hoja = libro.add_worksheet();
    hoja.set_name("Reservas")
        .map_err(|e| AppError::Excel(e.to_string()))?;

    // Cabecera en negrita
    let negrita = Format::new().set_bold();
    for (col, titulo) in CABECERAS.iter().enumerate() {
        hoja.write_string_with_format(0, col as u16, *titulo, &negrita)
            .map_err(|e| AppError::Excel(e.to_string()))?;
    }

    // Filas de datos
    for (i, det) in detalles.iter().enumerate() {
        let fila = (i + 1) as u32;
        let r = &det.reserva;
        let huespedes = det
            .huespedes
            .iter()
            .map(|h| h.nombre_completo())
            .collect::<Vec<_>>()
            .join(", ");

        let textos = [
            r.id.to_string(),
            det.titular.nombre_completo(),
            det.titular.documento.clone(),
            r.fecha_inicio.format("%Y-%m-%d").to_string(),
            r.fecha_fin.format("%Y-%m-%d").to_string(),
            det.habitaciones_resumen(),
            huespedes,
            r.estado.to_string(),
        ];
        for (col, texto) in textos.iter().enumerate() {
            hoja.write_string(fila, col as u16, texto)
                .map_err(|e| AppError::Excel(e.to_string()))?;
        }
        hoja.write_number(fila, 8, r.tarifa_total)
            .map_err(|e| AppError::Excel(e.to_string()))?;
        hoja.write_string(fila, 9, r.notas.as_str())
            .map_err(|e| AppError::Excel(e.to_string()))?;
    }

    // Asegura que la carpeta de destino existe antes de guardar
    if let Some(padre) = std::path::Path::new(ruta).parent() {
        if !padre.as_os_str().is_empty() {
            std::fs::create_dir_all(padre)?;
        }
    }
    libro.save(ruta).map_err(|e| AppError::Excel(e.to_string()))?;
    Ok(())
}
