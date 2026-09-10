use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub printer: PrinterConfig,
    #[serde(default)]
    pub printers: Vec<PrinterProfile>,
    #[serde(default)]
    pub active_printer_id: String,
    pub detection: DetectionConfig,
    pub notifications: NotificationsConfig,
    pub server: ServerConfig,
    pub logging: LoggingConfig,
    #[serde(default)]
    pub onboarding_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfig {
    pub ip: String,
    pub printer_id: String,
    pub pincode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterProfile {
    pub id: String,
    pub label: String,
    pub ip: String,
    pub printer_id: String,
    pub pincode: String,
}

impl PrinterProfile {
    pub fn from_config(id: String, cfg: &PrinterConfig) -> Self {
        Self {
            id,
            label: default_printer_label(&cfg.ip, &cfg.printer_id),
            ip: cfg.ip.clone(),
            printer_id: cfg.printer_id.clone(),
            pincode: cfg.pincode.clone(),
        }
    }

    pub fn to_config(&self) -> PrinterConfig {
        PrinterConfig {
            ip: self.ip.clone(),
            printer_id: self.printer_id.clone(),
            pincode: self.pincode.clone(),
        }
    }
}

pub fn default_printer_label(ip: &str, printer_id: &str) -> String {
    if !printer_id.is_empty() {
        format!("CC2 {printer_id}")
    } else if !ip.is_empty() {
        format!("CC2 {ip}")
    } else {
        "Elegoo CC2".to_string()
    }
}

pub fn printer_profile_id(ip: &str, printer_id: &str) -> String {
    if !printer_id.trim().is_empty() {
        format!("cc2-{}", printer_id.trim())
    } else {
        format!("cc2-{}", ip.trim().replace('.', "-"))
    }
}

pub fn sync_active_printer_profile(config: &mut AppConfig) {
    if config.active_printer_id.is_empty() && !config.printers.is_empty() {
        config.active_printer_id = config.printers[0].id.clone();
    }

    if let Some(active) = config
        .printers
        .iter()
        .find(|profile| profile.id == config.active_printer_id)
    {
        config.printer = active.to_config();
        return;
    }

    if !config.printer.ip.is_empty() {
        let id = printer_profile_id(&config.printer.ip, &config.printer.printer_id);
        config.active_printer_id = id.clone();
        config.printers.push(PrinterProfile::from_config(id, &config.printer));
    }
}

pub fn upsert_active_printer_profile(config: &mut AppConfig) {
    if config.printer.ip.is_empty() {
        return;
    }

    let id = if config.active_printer_id.is_empty() {
        printer_profile_id(&config.printer.ip, &config.printer.printer_id)
    } else {
        config.active_printer_id.clone()
    };
    config.active_printer_id = id.clone();

    let mut profile = PrinterProfile::from_config(id.clone(), &config.printer);
    if let Some(existing) = config.printers.iter().find(|p| p.id == id) {
        if !existing.label.trim().is_empty() {
            profile.label = existing.label.clone();
        }
    }

    if let Some(existing) = config.printers.iter_mut().find(|p| p.id == id) {
        *existing = profile;
    } else {
        config.printers.push(profile);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExcludeZone {
    /// norm 0..1
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl ExcludeZone {
    pub fn contains_center(&self, det_x1: f64, det_y1: f64, det_x2: f64, det_y2: f64) -> bool {
        let cx = (det_x1 + det_x2) / 2.0;
        let cy = (det_y1 + det_y2) / 2.0;
        cx >= self.x1 && cx <= self.x2 && cy >= self.y1 && cy <= self.y2
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionConfig {
    pub enabled: bool,
    pub interval_secs: u32,
    /// warn threshold
    #[serde(default = "default_notify_threshold")]
    pub notify_threshold: f64,
    /// pause threshold >= notify
    #[serde(alias = "threshold", default = "default_pause_threshold")]
    pub pause_threshold: f64,
    pub confirmation_frames: u32,
    pub obico_url: String,
    #[serde(default)]
    pub exclude_zones: Vec<ExcludeZone>,
}

fn default_notify_threshold() -> f64 { 0.5 }
fn default_pause_threshold() -> f64 { 0.7 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DestinationKind {
    Ntfy,
    Discord,
    Telegram,
    Webhook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventToggles {
    #[serde(default = "default_true")]
    pub print_started: bool,
    #[serde(default = "default_true")]
    pub print_finished: bool,
    #[serde(default = "default_true")]
    pub print_paused: bool,
    #[serde(default = "default_true")]
    pub failure_notify: bool,
    #[serde(default = "default_true")]
    pub failure_pause: bool,
    #[serde(default = "default_true")]
    pub auto_paused: bool,
    #[serde(default = "default_true")]
    pub camera_lost: bool,
    #[serde(default = "default_true")]
    pub camera_restored: bool,
    #[serde(default = "default_true")]
    pub emergency_stop: bool,
    #[serde(default = "default_true")]
    pub machine_error: bool,
    #[serde(default = "default_true")]
    pub id_not_match: bool,
    #[serde(default = "default_true")]
    pub auth_error: bool,
    #[serde(default = "default_true")]
    pub print_resumed: bool,
    #[serde(default = "default_true")]
    pub print_stopped: bool,
    #[serde(default = "default_true")]
    pub print_finished_ok: bool,
    #[serde(default)]
    pub progress_milestone: bool,
    #[serde(default = "default_true")]
    pub connected: bool,
    #[serde(default = "default_true")]
    pub disconnected: bool,
    #[serde(default = "default_true")]
    pub detection_engine_error: bool,
}

fn default_true() -> bool { true }

impl Default for EventToggles {
    fn default() -> Self {
        Self {
            print_started: true,
            print_finished: true,
            print_paused: true,
            failure_notify: true,
            failure_pause: true,
            auto_paused: true,
            camera_lost: true,
            camera_restored: true,
            emergency_stop: true,
            machine_error: true,
            id_not_match: true,
            auth_error: true,
            print_resumed: true,
            print_stopped: true,
            print_finished_ok: true,
            progress_milestone: false,
            connected: true,
            disconnected: true,
            detection_engine_error: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationDestination {
    pub id: String,
    pub kind: DestinationKind,
    pub enabled: bool,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ntfy_server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ntfy_topic: Option<String>,
    /// URL opened when user taps the notification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ntfy_tap_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discord_webhook_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_bot_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_chat_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub telegram_thread_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_url: Option<String>,
    /// 0 disables percentage notifications. Values are clamped to 5..100.
    #[serde(default)]
    pub progress_interval: u8,
    #[serde(default)]
    pub attach_snapshot: bool,
    #[serde(default)]
    pub toggles: EventToggles,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationsConfig {
    #[serde(default)]
    pub destinations: Vec<NotificationDestination>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}

impl AppConfig {
    pub fn printer_password(&self) -> &str {
        if !self.printer.pincode.is_empty() {
            &self.printer.pincode
        } else {
            "123456"
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            printer: PrinterConfig {
                ip: String::new(),
                printer_id: String::new(),
                pincode: String::new(),
            },
            printers: Vec::new(),
            active_printer_id: String::new(),
            detection: DetectionConfig {
                enabled: true,
                interval_secs: 15,
                notify_threshold: 0.5,
                pause_threshold: 0.7,
                confirmation_frames: 2,
                obico_url: "http://localhost:3333".to_string(),
                exclude_zones: Vec::new(),
            },
            notifications: NotificationsConfig {
                destinations: Vec::new(),
            },
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 8484,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
            },
            onboarding_complete: false,
        }
    }
}

/// pincode: 6 ascii alnum
pub fn validate_pincode(p: &str) -> Result<(), ConfigError> {
    if p.len() != 6 || !p.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(ConfigError::InvalidPincode);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> AppConfig {
        AppConfig::default()
    }

    #[test]
    fn validate_pincode_accepts_valid() {
        assert!(validate_pincode("ABC123").is_ok());
        assert!(validate_pincode("Abc123").is_ok());
        assert!(validate_pincode("abc123").is_ok());
        assert!(validate_pincode("000000").is_ok());
        assert!(validate_pincode("ZZZZZZ").is_ok());
    }

    #[test]
    fn validate_pincode_rejects_short_or_invalid() {
        assert!(validate_pincode("ABC12").is_err());
        assert!(validate_pincode("ABC1234").is_err());
        assert!(validate_pincode("AB-C12").is_err());
        assert!(validate_pincode("AB C12").is_err());
        assert!(validate_pincode("ABC12!").is_err());
    }

    #[test]
    fn printer_password_uses_pincode_when_set() {
        let mut cfg = base_config();
        cfg.printer.pincode = "Abc123".to_string();
        assert_eq!(cfg.printer_password(), "Abc123");
    }

    #[test]
    fn printer_password_defaults_to_123456() {
        let cfg = base_config();
        assert_eq!(cfg.printer_password(), "123456");
    }

    #[test]
    fn exclude_zone_contains_center() {
        let zone = ExcludeZone { x1: 0.2, y1: 0.2, x2: 0.8, y2: 0.8 };
        assert!(zone.contains_center(0.3, 0.3, 0.7, 0.7));
        assert!(!zone.contains_center(0.0, 0.0, 0.1, 0.1));
        assert!(!zone.contains_center(0.85, 0.85, 0.95, 0.95));
    }
}
