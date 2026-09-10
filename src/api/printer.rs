use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::Response;
use axum::Json;
use futures::TryStreamExt;
use serde::Deserialize;
use serde_json::Value;
use tracing::debug;

use super::router::AppState;
use crate::error::AppError;
use crate::printer::manager::StartPrintSlotMap;
use crate::printer::state::{normalize_machine_status, NormalizedStatus, PrintState};

#[derive(serde::Serialize)]
pub struct PrinterStatusResponse {
    pub connected: bool,
    pub connected_raw: bool,
    pub connected_ws: bool,
    pub printer_id: String,
    pub printer_ip: String,
    pub state: Value,
    pub phase: crate::printer::state::PhaseInfo,
}

pub async fn get_status(
    State(state): State<AppState>,
) -> Result<Json<PrinterStatusResponse>, AppError> {
    let printer_state = state.printer_state.read().await;
    let full = serde_json::to_value(&printer_state.full).unwrap_or(Value::Null);
    let connected = printer_state.connected;
    let connected_raw = printer_state.connected_raw;
    let connected_ws = printer_state.connected_ws;
    let phase = crate::printer::state::build_phase_info(
        printer_state.full.machine_status.status,
        printer_state.full.machine_status.sub_status,
        &printer_state.full.print_status.state,
    );
    drop(printer_state);

    Ok(Json(PrinterStatusResponse {
        connected,
        connected_raw,
        connected_ws,
        printer_id: state.manager.printer_id().await,
        printer_ip: state.manager.printer_ip().await,
        state: full,
        phase,
    }))
}

#[derive(Deserialize)]
pub struct PrintRequest {
    pub filename: String,
    pub storage_media: String,
    #[serde(default = "default_plate")]
    pub plate: String,
    #[serde(default)]
    pub tray_id: Option<i64>,
    /// 0-based canvas slot (`t` in slot_map)
    #[serde(default)]
    pub tray_slot: Option<i64>,
    #[serde(default)]
    pub canvas_id: i64,
    #[serde(default)]
    pub slot_map: Vec<StartPrintSlotMap>,
    #[serde(default)]
    pub timelapse: bool,
    #[serde(default = "default_true")]
    pub bedlevel_force: bool,
}

fn default_plate() -> String { "textured".to_string() }
fn default_true() -> bool { true }

pub async fn start_print(
    State(state): State<AppState>,
    Json(req): Json<PrintRequest>,
) -> Result<Json<Value>, AppError> {
    debug!("API: start_print {} plate={} canvas={} slot={:?} tray={:?}",
        req.filename, req.plate, req.canvas_id, req.tray_slot, req.tray_id);

    // fetch thumbnail pre-print; method 1045 often fails once print starts
    let needs_thumb = !state.printer_state.read().await
        .thumbnail_cache.contains_key(&req.filename);
    if needs_thumb {
        if let Ok(data) = state.manager.get_file_thumbnail("local", &req.filename).await {
            let thumb = data.get("thumbnail").and_then(|v| v.as_str()).unwrap_or("");
            if !thumb.is_empty() {
                state.printer_state.write().await
                    .thumbnail_cache.insert(req.filename.clone(), thumb.to_string());
            }
        }
    }

    state.manager.start_print(
        &req.filename, &req.storage_media, &req.plate,
        req.tray_id, req.tray_slot, req.canvas_id,
        req.slot_map,
        req.timelapse, req.bedlevel_force,
    ).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn pause_print(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    debug!("API: pause_print");
    state.manager.pause().await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn resume_print(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    debug!("API: resume_print");
    state.manager.resume().await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn stop_print(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    debug!("API: stop_print");
    state.manager.stop_print().await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct HomeRequest {
    pub axes: String,
}

pub async fn home_axes(
    State(state): State<AppState>,
    Json(req): Json<HomeRequest>,
) -> Result<Json<Value>, AppError> {
    let axes = req.axes.trim().to_lowercase();
    if axes.is_empty() || !axes.chars().all(|c| matches!(c, 'x' | 'y' | 'z')) {
        return Err(AppError::Validation("axes must be a non-empty combination of x, y, z".to_string()));
    }
    debug!("API: home_axes axes={axes}");
    state.manager.home_axes(&axes).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct JogRequest {
    pub axis: String,
    pub distance: f64,
}

pub async fn jog_axis(
    State(state): State<AppState>,
    Json(req): Json<JogRequest>,
) -> Result<Json<Value>, AppError> {
    let axis = req.axis.trim().to_lowercase();
    if !matches!(axis.as_str(), "x" | "y" | "z") {
        return Err(AppError::Validation("axis must be x, y, or z".to_string()));
    }
    if !req.distance.is_finite() || req.distance == 0.0 {
        return Err(AppError::Validation("distance must be a non-zero finite number".to_string()));
    }
    debug!("API: jog_axis axis={axis} distance={}", req.distance);
    state.manager.jog_axis(&axis, req.distance).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct LedRequest {
    pub power: i64,
}

pub async fn set_led(
    State(state): State<AppState>,
    Json(req): Json<LedRequest>,
) -> Result<Json<Value>, AppError> {
    debug!("API: set_led power={}", req.power);
    state.manager.set_led(req.power != 0).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct FanRequest {
    pub name: String,
    pub speed: u8,
}

pub async fn set_fan(
    State(state): State<AppState>,
    Json(req): Json<FanRequest>,
) -> Result<Json<Value>, AppError> {
    debug!("API: set_fan {}={}", req.name, req.speed);
    state.manager.set_fan(&req.name, req.speed).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct SpeedModeRequest {
    pub mode: u8,
}

pub async fn set_speed_mode(
    State(state): State<AppState>,
    Json(req): Json<SpeedModeRequest>,
) -> Result<Json<Value>, AppError> {
    debug!("API: set_speed_mode mode={}", req.mode);
    state.manager.set_speed_mode(req.mode).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct TemperatureRequest {
    #[serde(default)]
    pub nozzle: Option<i64>,
    #[serde(default)]
    pub bed: Option<i64>,
}

pub async fn set_temperatures(
    State(state): State<AppState>,
    Json(req): Json<TemperatureRequest>,
) -> Result<Json<Value>, AppError> {
    let nozzle = req.nozzle.map(|v| v.clamp(0, 350));
    let bed = req.bed.map(|v| v.clamp(0, 120));
    if nozzle.is_none() && bed.is_none() {
        return Err(AppError::Validation("nozzle or bed target is required".to_string()));
    }
    debug!("API: set_temperatures nozzle={nozzle:?} bed={bed:?}");
    state.manager.set_temperatures(nozzle, bed).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct FileListQuery {
    pub storage: Option<String>,
    // native page semantics (1-based)
    pub page_number: Option<i64>,
    pub page_size: Option<i64>,
    // legacy offset/limit  -  translated to page for backwards compat
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

pub async fn get_files(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<FileListQuery>,
) -> Result<Json<Value>, AppError> {
    let storage = query.storage.as_deref().unwrap_or("local");
    let (page_number, page_size) = if let (Some(pn), Some(ps)) = (query.page_number, query.page_size) {
        (pn.max(1), ps.max(1))
    } else {
        let page_size = query.limit.unwrap_or(50).max(1);
        let offset = query.offset.unwrap_or(0).max(0);
        let page_number = (offset / page_size) + 1;
        (page_number, page_size)
    };
    let result = state.manager.get_file_list(storage, page_number, page_size).await?;
    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct FileDetailQuery {
    pub storage: Option<String>,
    pub filename: String,
}

pub async fn get_file_detail(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<FileDetailQuery>,
) -> Result<Json<Value>, AppError> {
    if query.filename.is_empty() {
        return Err(AppError::Validation("filename is required".to_string()));
    }
    let storage = query.storage.as_deref().unwrap_or("local");
    let data = state.manager.get_file_info(storage, &query.filename).await?;
    Ok(Json(data))
}

pub async fn get_history(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let data = state.manager.get_print_history().await?;
    let history = data.get("history_task_list")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(Json(serde_json::json!({ "history": history })))
}

pub async fn canvas_refresh(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    debug!("API: canvas_refresh");
    let data = state.manager.canvas_refresh().await?;
    Ok(Json(data))
}

#[derive(Deserialize)]
pub struct AutoRefillRequest {
    pub enabled: bool,
}

pub async fn set_canvas_auto_refill(
    State(state): State<AppState>,
    Json(req): Json<AutoRefillRequest>,
) -> Result<Json<Value>, AppError> {
    debug!("API: set_canvas_auto_refill enabled={}", req.enabled);
    state.manager.set_ams_auto_refill(req.enabled).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct CanvasTrayRequest {
    pub canvas_id: i64,
    pub tray_id: i64,
    pub tray_slot: i64,
}

#[derive(Deserialize)]
pub struct CanvasSlotRequest {
    pub canvas_id: i64,
    pub tray_id: i64,
    pub tray_slot: i64,
    #[serde(default)]
    pub filament_name: Option<String>,
    #[serde(default)]
    pub filament_type: Option<String>,
    #[serde(default)]
    pub filament_color: Option<String>,
    #[serde(default)]
    pub brand: Option<String>,
    #[serde(default)]
    pub filament_code: Option<String>,
    #[serde(default)]
    pub min_nozzle_temp: Option<i64>,
    #[serde(default)]
    pub max_nozzle_temp: Option<i64>,
}

fn ensure_canvas_safe(ps: &crate::printer::state::PrinterState) -> Result<(), AppError> {
    if !ps.connected {
        return Err(crate::error::PrinterError::NotConnected.into());
    }
    if matches!(ps.print_state(), PrintState::Printing | PrintState::Paused) {
        return Err(AppError::Validation("Canvas changes are disabled during an active print".to_string()));
    }
    Ok(())
}

fn validate_canvas_ids(canvas_id: i64, tray_id: i64, tray_slot: i64) -> Result<(), AppError> {
    if canvas_id < 0 {
        return Err(AppError::Validation("canvas_id must be >= 0".to_string()));
    }
    if tray_id < 0 {
        return Err(AppError::Validation("tray_id must be >= 0".to_string()));
    }
    if !(0..=3).contains(&tray_slot) {
        return Err(AppError::Validation("tray_slot must be between 0 and 3".to_string()));
    }
    Ok(())
}

fn timelapse_status_allowed(status: &NormalizedStatus, print_state: &PrintState) -> bool {
    if matches!(print_state, PrintState::Printing | PrintState::Paused) {
        return false;
    }

    matches!(
        status,
        NormalizedStatus::Idle
            | NormalizedStatus::Printing
            | NormalizedStatus::PrintCompleted
            | NormalizedStatus::Canceled
    )
}

pub async fn canvas_load(
    State(state): State<AppState>,
    Json(req): Json<CanvasTrayRequest>,
) -> Result<Json<Value>, AppError> {
    validate_canvas_ids(req.canvas_id, req.tray_id, req.tray_slot)?;
    let ps = state.printer_state.read().await;
    ensure_canvas_safe(&*ps)?;
    drop(ps);
    debug!("API: canvas_load canvas={} tray={} slot={}", req.canvas_id, req.tray_id, req.tray_slot);
    state.manager.canvas_load(req.canvas_id, req.tray_id, req.tray_slot).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn canvas_unload(
    State(state): State<AppState>,
    Json(req): Json<CanvasTrayRequest>,
) -> Result<Json<Value>, AppError> {
    validate_canvas_ids(req.canvas_id, req.tray_id, req.tray_slot)?;
    let ps = state.printer_state.read().await;
    ensure_canvas_safe(&*ps)?;
    drop(ps);
    debug!("API: canvas_unload canvas={} tray={} slot={}", req.canvas_id, req.tray_id, req.tray_slot);
    state.manager.canvas_unload(req.canvas_id, req.tray_id, req.tray_slot).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn canvas_set_slot(
    State(state): State<AppState>,
    Json(req): Json<CanvasSlotRequest>,
) -> Result<Json<Value>, AppError> {
    validate_canvas_ids(req.canvas_id, req.tray_id, req.tray_slot)?;
    let ps = state.printer_state.read().await;
    ensure_canvas_safe(&*ps)?;
    drop(ps);

    let color = req
        .filament_color
        .unwrap_or_default()
        .trim()
        .trim_start_matches('#')
        .to_string();
    if !color.is_empty() && (color.len() != 6 || !color.chars().all(|c| c.is_ascii_hexdigit())) {
        return Err(AppError::Validation("filament_color must be a 6 digit hex color".to_string()));
    }

    let mut info = serde_json::Map::new();
    info.insert("filament_name".to_string(), serde_json::json!(req.filament_name.unwrap_or_default()));
    info.insert("filament_type".to_string(), serde_json::json!(req.filament_type.unwrap_or_default()));
    info.insert("filament_color".to_string(), serde_json::json!(color));
    info.insert("brand".to_string(), serde_json::json!(req.brand.unwrap_or_default()));
    info.insert("filament_code".to_string(), serde_json::json!(req.filament_code.unwrap_or_default()));
    if let Some(v) = req.min_nozzle_temp {
        info.insert("min_nozzle_temp".to_string(), serde_json::json!(v.clamp(0, 350)));
    }
    if let Some(v) = req.max_nozzle_temp {
        info.insert("max_nozzle_temp".to_string(), serde_json::json!(v.clamp(0, 350)));
    }

    debug!("API: canvas_set_slot canvas={} tray={} slot={}", req.canvas_id, req.tray_id, req.tray_slot);
    state.manager.canvas_set_tray_info(
        req.canvas_id,
        req.tray_id,
        req.tray_slot,
        serde_json::Value::Object(info),
    ).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Deserialize)]
pub struct ThumbnailQuery {
    pub storage: Option<String>,
    pub filename: String,
}

pub async fn get_thumbnail(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<ThumbnailQuery>,
) -> Result<Json<Value>, AppError> {
    if query.filename.is_empty() {
        return Err(AppError::Validation("filename is required".to_string()));
    }
    let storage = query.storage.as_deref().unwrap_or("local");

    {
        let ps = state.printer_state.read().await;
        if let Some(cached) = ps.thumbnail_cache.get(&query.filename) {
            return Ok(Json(serde_json::json!({
                "thumbnail": cached,
                "filename": query.filename,
            })));
        }
    }

    let data = state.manager.get_file_thumbnail(storage, &query.filename).await?;
    let thumbnail = data.get("thumbnail").and_then(|v| v.as_str()).unwrap_or("").to_string();

    if !thumbnail.is_empty() {
        state.printer_state.write().await
            .thumbnail_cache.insert(query.filename.clone(), thumbnail.clone());
    }

    Ok(Json(serde_json::json!({
        "thumbnail": thumbnail,
        "filename": query.filename,
    })))
}

#[derive(Deserialize)]
pub struct TimelapseQuery {
    pub path: String,
}

pub async fn download_timelapse(
    State(state): State<AppState>,
    Query(query): Query<TimelapseQuery>,
) -> Result<Response<Body>, AppError> {
    let raw_path = query.path.trim().trim_start_matches('/');
    if raw_path.is_empty() || raw_path.contains("..") || raw_path.contains('\\') {
        return Err(AppError::Validation("invalid timelapse path".to_string()));
    }
    if !(raw_path.starts_with("video/") || raw_path.starts_with("picture/")) {
        return Err(AppError::Validation("timelapse path must start with video/ or picture/".to_string()));
    }

    {
        let ps = state.printer_state.read().await;
        if ps.connected {
            let machine_status = normalize_machine_status(
                ps.full.machine_status.status,
                ps.full.machine_status.sub_status,
            );
            let print_state = ps.print_state();
            if !timelapse_status_allowed(&machine_status, &print_state) {
                return Err(AppError::Validation(format!(
                    "Timelapse download is only available when the printer is idle (current status: {})",
                    machine_status.label()
                )));
            }
        }
    }

    let (printer_ip, token) = {
        let config = state.config.read().await;
        (
            config.printer.ip.trim().to_string(),
            config.printer_password().to_string(),
        )
    };
    if printer_ip.trim().is_empty() {
        return Err(crate::error::PrinterError::NotConnected.into());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(crate::error::PrinterError::from)?;
    let mut candidates = Vec::new();
    let mut download_url = reqwest::Url::parse(&format!("http://{printer_ip}/download"))
        .map_err(|_| AppError::Validation("invalid printer url".to_string()))?;
    download_url
        .query_pairs_mut()
        .append_pair("X-Token", &token)
        .append_pair("file_name", raw_path);
    candidates.push(download_url);

    for base in [
        format!("http://{printer_ip}/downloadFile/"),
        format!("http://{printer_ip}/download/"),
        format!("http://{printer_ip}/"),
    ] {
        let mut url = reqwest::Url::parse(&base)
            .map_err(|_| AppError::Validation("invalid printer url".to_string()))?;
        let prefix = url.path().trim_matches('/');
        let path = if prefix.is_empty() {
            raw_path.to_string()
        } else {
            format!("{prefix}/{raw_path}")
        };
        url.set_path(&path);
        candidates.push(url);
    }

    let mut last_status = None;
    let mut selected = None;
    for base in candidates {
        let mut request = client.get(base);
        if !token.is_empty() {
            request = request.header("X-Token", &token);
        }
        let resp = request.send().await.map_err(crate::error::PrinterError::from)?;
        if !resp.status().is_success() {
            last_status = Some(resp.status());
            continue;
        }
        let content_type = resp
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        if content_type.contains("multipart/x-mixed-replace") {
            last_status = Some(StatusCode::BAD_GATEWAY);
            continue;
        }
        selected = Some(resp);
        break;
    }
    let Some(resp) = selected else {
        let status = last_status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "no response".to_string());
        return Err(AppError::Validation(format!("timelapse unavailable: {status}")));
    };

    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .cloned()
        .unwrap_or_else(|| HeaderValue::from_static("application/octet-stream"));
    let filename = raw_path
        .rsplit('/')
        .next()
        .filter(|v| !v.is_empty())
        .unwrap_or("timelapse.mp4")
        .replace('"', "'");
    let stream = resp
        .bytes_stream()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e));

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type)
        .header(header::CACHE_CONTROL, "no-store")
        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{filename}\""))
        .body(Body::from_stream(stream))
        .map_err(|e| AppError::Validation(format!("failed to build response: {e}")))
}
