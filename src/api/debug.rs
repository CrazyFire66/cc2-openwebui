use axum::extract::State;
use axum::Json;
use serde::Serialize;

use super::router::AppState;
use crate::error::AppError;

#[derive(Serialize)]
pub struct DebugResponse {
    pub version: &'static str,
    pub configured: bool,
    pub active_printer_id: String,
    pub printer_count: usize,
    pub printer_ip: String,
    pub printer_id: String,
    pub connected: bool,
    pub connected_raw: bool,
    pub connected_ws: bool,
    pub detected_subnets: Vec<String>,
    pub detection_enabled: bool,
    pub obico_url: String,
    pub notification_destinations: usize,
}

pub async fn get_debug(State(state): State<AppState>) -> Result<Json<DebugResponse>, AppError> {
    let config = state.config.read().await;
    let printer_state = state.printer_state.read().await;

    Ok(Json(DebugResponse {
        version: env!("CARGO_PKG_VERSION"),
        configured: !config.printer.ip.is_empty(),
        active_printer_id: config.active_printer_id.clone(),
        printer_count: config.printers.len(),
        printer_ip: config.printer.ip.clone(),
        printer_id: config.printer.printer_id.clone(),
        connected: printer_state.connected,
        connected_raw: printer_state.connected_raw,
        connected_ws: printer_state.connected_ws,
        detected_subnets: crate::api::setup::detect_local_subnets(),
        detection_enabled: config.detection.enabled,
        obico_url: config.detection.obico_url.clone(),
        notification_destinations: config.notifications.destinations.len(),
    }))
}
