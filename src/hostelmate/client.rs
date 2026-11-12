//! Cliente HTTP de la API de HostelMate.
//!
//! IMPORTANTE: los endpoints reales de HostelMate no son conocidos todavía.
//! Todas las rutas y payloads son PLACEHOLDER marcados con
//! `// TODO(HostelMate):` para ajustarlos a la documentación oficial.
//!
//! Toda request saliente lleva `Authorization: Bearer <api_key>` e
//! `Idempotency-Key: <sync_id uuid>` para evitar duplicados en reintentos.

use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::models::ReservaDetalle;

/// Payload de reserva intercambiado con HostelMate (saliente y entrante).
/// TODO(HostelMate): ajustar campos al esquema real de la API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservaExterna {
    pub sync_id: String,
    pub titular_nombre: String,
    #[serde(default)]
    pub titular_documento: String,
    pub fecha_inicio: NaiveDate,
    pub fecha_fin: NaiveDate,
    /// Estado como texto ('pendiente'|'confirmada'|'checkin'|'cancelada')
    pub estado: String,
    #[serde(default)]
    pub tarifa_total: f64,
    #[serde(default)]
    pub notas: String,
    #[serde(default)]
    pub habitaciones: Vec<String>,
}

impl From<&ReservaDetalle> for ReservaExterna {
    fn from(d: &ReservaDetalle) -> Self {
        Self {
            sync_id: d.reserva.sync_id.clone(),
            titular_nombre: d.titular.nombre_completo(),
            titular_documento: d.titular.documento.clone(),
            fecha_inicio: d.reserva.fecha_inicio,
            fecha_fin: d.reserva.fecha_fin,
            estado: d.reserva.estado.to_string(),
            tarifa_total: d.reserva.tarifa_total,
            notas: d.reserva.notas.clone(),
            habitaciones: d.habitaciones.iter().map(|h| h.numero.clone()).collect(),
        }
    }
}

/// Interfaz de la API de HostelMate (permite mockear en tests).
/// Usa `async fn` nativo en traits (estable desde Rust 1.75).
pub trait HostelMateApi: Send + Sync {
    async fn crear_reserva(&self, reserva: &ReservaDetalle) -> AppResult<()>;
    async fn actualizar_reserva(&self, reserva: &ReservaDetalle) -> AppResult<()>;
    async fn cancelar_reserva(&self, sync_id: &str) -> AppResult<()>;
    async fn listar_reservas_desde(&self, desde: NaiveDateTime) -> AppResult<Vec<ReservaExterna>>;
}

/// Cliente HTTP de HostelMate basado en reqwest.
pub struct HostelMateHttpClient {
    base_url: String,
    api_key: String,
    http: reqwest::Client,
    /// false si no hay API key configurada: las llamadas se omiten con log
    habilitado: bool,
}

impl HostelMateHttpClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        let habilitado = !api_key.trim().is_empty() && api_key != "pega_tu_api_key_aqui";
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            http: reqwest::Client::new(),
            habilitado,
        }
    }

    /// Indica si el cliente tiene credenciales para llamar a la API real.
    pub fn habilitado(&self) -> bool {
        self.habilitado
    }

    /// Request autenticada con clave de idempotencia.
    fn request(&self, metodo: reqwest::Method, url: &str, sync_id: &str) -> reqwest::RequestBuilder {
        self.http
            .request(metodo, url)
            .bearer_auth(&self.api_key)
            .header("Idempotency-Key", sync_id)
    }

    /// Ejecuta una request esperando solo éxito (sin cuerpo de respuesta).
    async fn ejecutar(&self, req: reqwest::RequestBuilder) -> AppResult<()> {
        if !self.habilitado {
            tracing::debug!("HostelMate deshabilitado (sin API key): llamada omitida");
            return Ok(());
        }
        let resp = req
            .send()
            .await
            .map_err(|e| AppError::HostelMate(format!("error de red: {}", e)))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let cuerpo = resp.text().await.unwrap_or_default();
            return Err(AppError::HostelMate(format!(
                "respuesta {} de HostelMate: {}",
                status, cuerpo
            )));
        }
        Ok(())
    }
}

impl HostelMateApi for HostelMateHttpClient {
    /// TODO(HostelMate): ajustar ruta y payload según documentación oficial.
    async fn crear_reserva(&self, reserva: &ReservaDetalle) -> AppResult<()> {
        let url = format!("{}/api/reservas", self.base_url);
        let payload = ReservaExterna::from(reserva);
        self.ejecutar(
            self.request(reqwest::Method::POST, &url, &reserva.reserva.sync_id)
                .json(&payload),
        )
        .await
    }

    /// TODO(HostelMate): ajustar ruta y payload según documentación oficial.
    async fn actualizar_reserva(&self, reserva: &ReservaDetalle) -> AppResult<()> {
        let url = format!("{}/api/reservas/{}", self.base_url, reserva.reserva.sync_id);
        let payload = ReservaExterna::from(reserva);
        self.ejecutar(
            self.request(reqwest::Method::PUT, &url, &reserva.reserva.sync_id)
                .json(&payload),
        )
        .await
    }

    /// TODO(HostelMate): ajustar ruta (podría ser DELETE o POST cancel).
    async fn cancelar_reserva(&self, sync_id: &str) -> AppResult<()> {
        let url = format!("{}/api/reservas/{}/cancel", self.base_url, sync_id);
        self.ejecutar(self.request(reqwest::Method::POST, &url, sync_id))
            .await
    }

    /// TODO(HostelMate): ajustar parámetros de consulta y mapeo de respuesta.
    async fn listar_reservas_desde(&self, desde: NaiveDateTime) -> AppResult<Vec<ReservaExterna>> {
        if !self.habilitado {
            return Ok(Vec::new());
        }
        let url = format!(
            "{}/api/reservas?desde={}",
            self.base_url,
            desde.format("%Y-%m-%dT%H:%M:%S")
        );
        let resp = self
            .http
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| AppError::HostelMate(format!("error de red: {}", e)))?;
        if !resp.status().is_success() {
            return Err(AppError::HostelMate(format!(
                "respuesta {} de HostelMate",
                resp.status()
            )));
        }
        let reservas = resp
            .json::<Vec<ReservaExterna>>()
            .await
            .map_err(|e| AppError::HostelMate(format!("JSON inválido: {}", e)))?;
        Ok(reservas)
    }
}
