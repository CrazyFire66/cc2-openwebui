use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

use super::router::AppState;
use crate::config::{
    printer_profile_id, sync_active_printer_profile, upsert_active_printer_profile,
    validate_pincode, AppConfig, PrinterProfile,
};
use crate::error::{AppError, ConfigError};

#[derive(Serialize)]
pub struct SettingsExport {
    pub version: &'static str,
    pub exported_at: u64,
    pub config: AppConfig,
}

#[derive(Deserialize)]
pub struct SettingsImport {
    pub config: AppConfig,
}

pub async fn get_settings(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let config = state.config.read().await;
    let value = serde_json::to_value(&*config).unwrap_or(Value::Null);
    Ok(Json(value))
}

/// update config + save
pub async fn update_settings(
    State(state): State<AppState>,
    Json(req): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let mut config = state.config.write().await;
    let previous_printer = config.printer.clone();

    if let Some(profiles_value) = req.get("printers") {
        let mut profiles: Vec<PrinterProfile> = serde_json::from_value(profiles_value.clone())
            .map_err(|e| AppError::Validation(format!("invalid printer profiles: {e}")))?;
        profiles.retain(|profile| !profile.ip.trim().is_empty());
        for profile in &mut profiles {
            profile.id = if profile.id.trim().is_empty() {
                printer_profile_id(&profile.ip, &profile.printer_id)
            } else {
                profile.id.trim().to_string()
            };
            profile.label = profile.label.trim().to_string();
            profile.ip = profile.ip.trim().to_string();
            profile.printer_id = profile.printer_id.trim().to_string();
            profile.pincode = profile.pincode.trim().to_string();
            if profile.label.is_empty() {
                profile.label = crate::config::default_printer_label(&profile.ip, &profile.printer_id);
            }
            if !profile.pincode.is_empty() {
                validate_pincode(&profile.pincode)?;
            }
        }
        profiles.sort_by(|a, b| a.id.cmp(&b.id));
        profiles.dedup_by(|a, b| a.id == b.id);
        config.printers = profiles;
    }

    if let Some(active_id) = req.get("active_printer_id").and_then(|v| v.as_str()) {
        config.active_printer_id = active_id.trim().to_string();
        if let Some(active) = config.printers.iter().find(|p| p.id == config.active_printer_id) {
            config.printer = active.to_config();
        }
    }

    if let Some(printer) = req.get("printer") {
        if let Some(ip) = printer.get("ip").and_then(|v| v.as_str()) {
            config.printer.ip = ip.trim().to_string();
        }
        if let Some(printer_id) = printer.get("printer_id").and_then(|v| v.as_str()) {
            config.printer.printer_id = printer_id.trim().to_string();
        }
        if let Some(pincode) = printer.get("pincode").and_then(|v| v.as_str()) {
            let normalized = pincode.trim().to_string();
            if !normalized.is_empty() {
                validate_pincode(&normalized)?;
            }
            config.printer.pincode = normalized;
        }
        upsert_active_printer_profile(&mut config);
    }

    if let Some(detection) = req.get("detection") {
        if let Some(v) = detection.get("enabled").and_then(|v| v.as_bool()) {
            config.detection.enabled = v;
        }
        if let Some(v) = detection.get("notify_threshold").and_then(|v| v.as_f64()) {
            config.detection.notify_threshold = v.clamp(0.0, 1.0);
        }
        if let Some(v) = detection.get("pause_threshold").and_then(|v| v.as_f64()) {
            config.detection.pause_threshold = v.clamp(0.0, 1.0);
        }
        if let Some(v) = detection.get("interval_secs").and_then(|v| v.as_u64()) {
            config.detection.interval_secs = (v as u32).max(5);
        }
        if let Some(v) = detection.get("obico_url").and_then(|v| v.as_str()) {
            config.detection.obico_url = v.to_string();
        }
    }

    if let Some(server) = req.get("server") {
        if let Some(v) = server.get("host").and_then(|v| v.as_str()) {
            config.server.host = v.to_string();
        }
        if let Some(v) = server.get("port").and_then(|v| v.as_u64()) {
            config.server.port = v as u16;
        }
    }

    if let Some(v) = req.get("logging").and_then(|l| l.get("level")).and_then(|v| v.as_str()) {
        config.logging.level = v.to_string();
    }

    let det_config = config.detection.clone();
    let det_enabled = config.detection.enabled;
    upsert_active_printer_profile(&mut config);
    let printer_cfg = config.printer.clone();
    let printer_profiles = config.printers.clone();
    let active_printer_id = config.active_printer_id.clone();
    let host = config.server.host.clone();
    let port = config.server.port;
    let log_level = config.logging.level.clone();
    let onboarding_complete = config.onboarding_complete;
    let config_snapshot = config.clone();
    drop(config);

    if let Err(e) = crate::db::save_printer_config(&state.db, &printer_cfg).await {
        warn!("failed to persist printer config: {e}");
        return Err(AppError::Config(ConfigError::Db(e)));
    }
    if let Err(e) = crate::db::save_printer_profiles(&state.db, &printer_profiles, &active_printer_id).await {
        warn!("failed to persist printer profiles: {e}");
        return Err(AppError::Config(ConfigError::Db(e)));
    }
    if let Err(e) = crate::db::save_detection_config(&state.db, &det_config).await {
        warn!("failed to persist detection config: {e}");
        return Err(AppError::Config(ConfigError::Db(e)));
    }
    if let Err(e) = crate::db::save_server_config(&state.db, &host, port, &log_level, onboarding_complete).await {
        warn!("failed to persist server config: {e}");
        return Err(AppError::Config(ConfigError::Db(e)));
    }

    let _ = state.det_config_tx.send(det_config);
    let _ = state.det_enabled_tx.send(det_enabled);

    if printer_cfg.ip != previous_printer.ip
        || printer_cfg.printer_id != previous_printer.printer_id
        || printer_cfg.pincode != previous_printer.pincode
    {
        state.manager.update_config(config_snapshot).await;
        if !printer_cfg.ip.is_empty() {
            if let Err(e) = state.manager.start().await {
                warn!("failed to restart printer manager after printer settings update: {e}");
                return Err(AppError::Printer(e));
            }
        }
        let _ = state.camera_ip_tx.send(printer_cfg.ip.clone());
    }

    Ok(Json(Value::Null))
}

pub async fn export_settings(State(state): State<AppState>) -> Result<Json<SettingsExport>, AppError> {
    let config = state.config.read().await.clone();
    let exported_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();

    Ok(Json(SettingsExport {
        version: env!("CARGO_PKG_VERSION"),
        exported_at,
        config,
    }))
}

pub async fn import_settings(
    State(state): State<AppState>,
    Json(mut req): Json<SettingsImport>,
) -> Result<Json<Value>, AppError> {
    validate_import_config(&mut req.config)?;

    {
        let mut config = state.config.write().await;
        *config = req.config.clone();
    }

    persist_full_config(&state, &req.config).await?;

    let _ = state.det_config_tx.send(req.config.detection.clone());
    let _ = state.det_enabled_tx.send(req.config.detection.enabled);
    let _ = state.camera_ip_tx.send(req.config.printer.ip.clone());
    state.manager.update_config(req.config.clone()).await;
    if !req.config.printer.ip.is_empty() {
        state.manager.start().await?;
    }

    Ok(Json(serde_json::json!({ "success": true })))
}

fn validate_import_config(config: &mut AppConfig) -> Result<(), AppError> {
    config.printer.ip = config.printer.ip.trim().to_string();
    config.printer.printer_id = config.printer.printer_id.trim().to_string();
    config.printer.pincode = config.printer.pincode.trim().to_string();
    if !config.printer.pincode.is_empty() {
        validate_pincode(&config.printer.pincode)?;
    }

    for profile in &mut config.printers {
        profile.id = profile.id.trim().to_string();
        profile.label = profile.label.trim().to_string();
        profile.ip = profile.ip.trim().to_string();
        profile.printer_id = profile.printer_id.trim().to_string();
        profile.pincode = profile.pincode.trim().to_string();
        if !profile.pincode.is_empty() {
            validate_pincode(&profile.pincode)?;
        }
    }
    sync_active_printer_profile(config);
    upsert_active_printer_profile(config);
    Ok(())
}

async fn persist_full_config(state: &AppState, config: &AppConfig) -> Result<(), AppError> {
    crate::db::reset_config(&state.db)
        .await
        .map_err(|e| AppError::Config(ConfigError::Db(e)))?;
    crate::db::save_printer_config(&state.db, &config.printer)
        .await
        .map_err(|e| AppError::Config(ConfigError::Db(e)))?;
    crate::db::save_printer_profiles(&state.db, &config.printers, &config.active_printer_id)
        .await
        .map_err(|e| AppError::Config(ConfigError::Db(e)))?;
    crate::db::save_detection_config(&state.db, &config.detection)
        .await
        .map_err(|e| AppError::Config(ConfigError::Db(e)))?;
    crate::db::save_server_config(
        &state.db,
        &config.server.host,
        config.server.port,
        &config.logging.level,
        config.onboarding_complete,
    )
    .await
    .map_err(|e| AppError::Config(ConfigError::Db(e)))?;
    for dest in &config.notifications.destinations {
        crate::db::upsert_destination(&state.db, dest)
            .await
            .map_err(|e| AppError::Config(ConfigError::Db(e)))?;
    }
    Ok(())
}
