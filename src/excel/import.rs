//! Importación de reservas desde Excel (.xlsx) con calamine.
//!
//! El mapeo campo -> columna se lee de la tabla `excel_column_map` y puede
//! ser una letra de columna ("A", "B", ...) o el nombre de la cabecera.
//!
//! TODO: el layout real del Excel del usuario puede variar (hojas, cabeceras,
//! formatos de fecha); el mapeo configurable en BD está pensado para
//! resolverlo sin tocar código.

use calamine::{open_workbook_auto, Data, Reader};
use chrono::{Duration, NaiveDate};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::models::{EstadoReserva, ReservaInput};
use crate::repos;
use crate::services::reservas_service;
use crate::state::AppState;

/// Reserva leída del Excel (vista previa / validación previa al insert).
#[derive(Debug, Clone, Default)]
pub struct ReservaImportada {
    pub titular_nombre: String,
    pub titular_documento: String,
    pub fecha_inicio: Option<NaiveDate>,
    pub fecha_fin: Option<NaiveDate>,
    pub habitacion: String,
    pub tarifa: f64,
    pub estado: String,
    pub notas: String,
    /// Errores de validación de la fila (vacío = lista para importar)
    pub errores: Vec<String>,
}

/// Campos internos esperados en el mapeo `excel_column_map`.
const CAMPOS: [&str; 8] = [
    "titular_nombre",
    "titular_documento",
    "fecha_inicio",
    "fecha_fin",
    "habitacion",
    "tarifa",
    "estado",
    "notas",
];

/// Convierte una letra de columna Excel ("A", "B", ..., "AA", ...) a índice 0-based.
fn letra_a_indice(letra: &str) -> Option<usize> {
    let letra = letra.trim().to_uppercase();
    if letra.is_empty() || !letra.chars().all(|c| c.is_ascii_uppercase()) {
        return None;
    }
    let mut indice = 0usize;
    for c in letra.chars() {
        indice = indice * 26 + (c as usize - 'A' as usize + 1);
    }
    Some(indice - 1)
}

/// Resuelve el índice de columna de cada campo a partir del mapeo en BD
/// y de la fila de cabeceras del Excel.
fn resolver_columnas(
    mapa: &[crate::models::ExcelColumnMap],
    cabeceras: &[String],
) -> HashMap<String, usize> {
    let mut indices = HashMap::new();
    for (pos, campo) in CAMPOS.iter().enumerate() {
        let valor = mapa
            .iter()
            .find(|m| m.campo == *campo)
            .map(|m| m.columna_excel.clone())
            .unwrap_or_else(|| {
                // Por defecto: columnas consecutivas A, B, C, ...
                letra_indice_a_string(pos)
            });
        if let Some(idx) = letra_a_indice(&valor) {
            indices.insert(campo.to_string(), idx);
        } else if let Some(idx) = cabeceras
            .iter()
            .position(|c| c.trim().eq_ignore_ascii_case(valor.trim()))
        {
            indices.insert(campo.to_string(), idx);
        }
    }
    indices
}

/// Convierte un índice 0-based a letra de columna Excel.
fn letra_indice_a_string(mut idx: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (idx % 26) as u8) as char);
        if idx < 26 {
            break;
        }
        idx = idx / 26 - 1;
    }
    s
}

/// Extrae el texto de una celda calamine (vacío si no aplica).
fn celda_texto(celda: Option<&Data>) -> String {
    match celda {
        Some(Data::String(s)) => s.trim().to_string(),
        Some(Data::Float(f)) => {
            if f.fract() == 0.0 {
                format!("{}", *f as i64)
            } else {
                format!("{}", f)
            }
        }
        Some(Data::Int(i)) => i.to_string(),
        Some(Data::Bool(b)) => b.to_string(),
        Some(Data::DateTime(dt)) => {
            // Fecha de Excel (serial) -> ISO 'YYYY-MM-DD'
            let base = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap_or_default();
            let fecha = base + Duration::days(dt.as_f64() as i64);
            fecha.format("%Y-%m-%d").to_string()
        }
        _ => String::new(),
    }
}

/// Parsea una fecha en formato ISO o 'DD/MM/YYYY'.
fn parsear_fecha(texto: &str) -> Option<NaiveDate> {
    let texto = texto.trim();
    if texto.is_empty() {
        return None;
    }
    NaiveDate::parse_from_str(texto, "%Y-%m-%d")
        .ok()
        .or_else(|| NaiveDate::parse_from_str(texto, "%d/%m/%Y").ok())
}

/// Lee el archivo Excel y devuelve las reservas detectadas (vista previa).
/// La primera hoja debe contener una fila de cabeceras en la fila 1.
pub async fn importar_reservas(db: &SqlitePool, ruta: &str) -> AppResult<Vec<ReservaImportada>> {
    let mapa = repos::config::listar_excel_map(db).await?;

    let mut libro = open_workbook_auto(ruta)
        .map_err(|e: calamine::Error| AppError::Excel(e.to_string()))?;
    let rango = libro
        .worksheet_range_at(0)
        .ok_or_else(|| AppError::Excel("el Excel no tiene hojas".to_string()))?
        .map_err(|e: calamine::Error| AppError::Excel(e.to_string()))?;

    let filas: Vec<Vec<Data>> = rango.rows().map(|f| f.to_vec()).collect();
    if filas.is_empty() {
        return Ok(Vec::new());
    }

    // Cabeceras (fila 0) para resolver columnas por nombre
    let cabeceras: Vec<String> = filas[0].iter().map(|c| celda_texto(Some(c))).collect();
    let columnas = resolver_columnas(&mapa, &cabeceras);

    let obtener = |fila: &[Data], campo: &str| -> String {
        columnas
            .get(campo)
            .and_then(|&idx| fila.get(idx))
            .map(|c| celda_texto(Some(c)))
            .unwrap_or_default()
    };

    let mut resultado = Vec::new();
    for (n_fila, fila) in filas.iter().enumerate().skip(1) {
        let titular_nombre = obtener(fila, "titular_nombre");
        // Salta filas completamente vacías
        if titular_nombre.is_empty()
            && obtener(fila, "fecha_inicio").is_empty()
            && obtener(fila, "habitacion").is_empty()
        {
            continue;
        }

        let mut item = ReservaImportada {
            titular_nombre,
            titular_documento: obtener(fila, "titular_documento"),
            fecha_inicio: parsear_fecha(&obtener(fila, "fecha_inicio")),
            fecha_fin: parsear_fecha(&obtener(fila, "fecha_fin")),
            habitacion: obtener(fila, "habitacion"),
            tarifa: obtener(fila, "tarifa").replace(',', ".").parse().unwrap_or(0.0),
            estado: {
                let e = obtener(fila, "estado").to_lowercase();
                if e.is_empty() {
                    "confirmada".to_string()
                } else {
                    e
                }
            },
            notas: obtener(fila, "notas"),
            errores: Vec::new(),
        };

        // Validación básica de la fila
        if item.titular_nombre.is_empty() {
            item.errores.push(format!("fila {}: falta titular", n_fila + 1));
        }
        if item.fecha_inicio.is_none() {
            item.errores
                .push(format!("fila {}: fecha_inicio inválida", n_fila + 1));
        }
        if item.fecha_fin.is_none() {
            item.errores
                .push(format!("fila {}: fecha_fin inválida", n_fila + 1));
        }
        if let (Some(ini), Some(fin)) = (item.fecha_inicio, item.fecha_fin) {
            if ini >= fin {
                item.errores
                    .push(format!("fila {}: check-in posterior al check-out", n_fila + 1));
            }
        }
        if item.habitacion.is_empty() {
            item.errores
                .push(format!("fila {}: falta habitación", n_fila + 1));
        }
        if item.estado.parse::<EstadoReserva>().is_err() {
            item.errores.push(format!(
                "fila {}: estado '{}' desconocido",
                n_fila + 1,
                item.estado
            ));
        }
        resultado.push(item);
    }
    Ok(resultado)
}

/// Inserta las reservas importadas válidas usando el servicio (valida
/// solapamiento). Devuelve (número importadas, lista de errores).
pub async fn insertar_importadas(
    state: &Arc<AppState>,
    items: Vec<ReservaImportada>,
) -> AppResult<(usize, Vec<String>)> {
    let mut importadas = 0usize;
    let mut errores: Vec<String> = Vec::new();

    for item in items {
        if !item.errores.is_empty() {
            errores.extend(item.errores.iter().cloned());
            continue;
        }
        let (Some(ini), Some(fin)) = (item.fecha_inicio, item.fecha_fin) else {
            continue;
        };

        // Buscar o crear el cliente titular por documento (o por nombre si no hay)
        let cliente_id: i64 = if !item.titular_documento.is_empty() {
            match sqlx::query_scalar::<_, i64>(
                "SELECT id FROM clientes WHERE documento = ?1 LIMIT 1",
            )
            .bind(&item.titular_documento)
            .fetch_optional(&state.db)
            .await?
            {
                Some(id) => id,
                None => {
                    repos::clientes::crear(
                        &state.db,
                        &item.titular_nombre,
                        "",
                        &item.titular_documento,
                        "",
                        "",
                    )
                    .await?
                    .id
                }
            }
        } else {
            repos::clientes::crear(&state.db, &item.titular_nombre, "", "", "", "")
                .await?
                .id
        };

        // Resolver habitación por número (acepta varias separadas por coma)
        let mut habitacion_ids = Vec::new();
        let mut faltan = Vec::new();
        for numero in item.habitacion.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            match sqlx::query_scalar::<_, i64>("SELECT id FROM habitaciones WHERE numero = ?1")
                .bind(numero)
                .fetch_optional(&state.db)
                .await?
            {
                Some(id) => habitacion_ids.push(id),
                None => faltan.push(numero.to_string()),
            }
        }
        if !faltan.is_empty() {
            errores.push(format!(
                "titular '{}': habitaciones no encontradas: {}",
                item.titular_nombre,
                faltan.join(", ")
            ));
            continue;
        }

        let estado = item
            .estado
            .parse::<EstadoReserva>()
            .unwrap_or(EstadoReserva::Confirmada);
        let input = ReservaInput {
            cliente_id,
            fecha_inicio: ini,
            fecha_fin: fin,
            estado,
            tarifa_total: item.tarifa,
            notas: item.notas.clone(),
            habitacion_ids,
            huesped_ids: Vec::new(),
        };

        match reservas_service::crear_reserva(state, &input, "importacion-excel").await {
            Ok(_) => importadas += 1,
            Err(e) => errores.push(format!("titular '{}': {}", item.titular_nombre, e)),
        }
    }
    Ok((importadas, errores))
}
