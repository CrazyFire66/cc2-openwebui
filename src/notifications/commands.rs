use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::camera::FrameBuffer;
use crate::config::{AppConfig, DestinationKind, NotificationDestination};
use crate::printer::state::PrinterState;

use super::telegram;

pub struct TelegramCommandPoller {
    state: Arc<RwLock<PrinterState>>,
    config: Arc<RwLock<AppConfig>>,
    frame_buffer: FrameBuffer,
    offsets: HashMap<String, i64>,
    initialized: HashSet<String>,
}

impl TelegramCommandPoller {
    pub fn new(
        state: Arc<RwLock<PrinterState>>,
        config: Arc<RwLock<AppConfig>>,
        frame_buffer: FrameBuffer,
    ) -> Self {
        Self {
            state,
            config,
            frame_buffer,
            offsets: HashMap::new(),
            initialized: HashSet::new(),
        }
    }

    pub async fn run(mut self) {
        loop {
            let destinations = self.telegram_destinations().await;
            if destinations.is_empty() {
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }

            for dest in destinations {
                if let Err(e) = self.poll_destination(&dest).await {
                    warn!("[telegram-commands] '{}' failed: {e}", dest.label);
                }
            }

            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    async fn telegram_destinations(&self) -> Vec<NotificationDestination> {
        self.config
            .read()
            .await
            .notifications
            .destinations
            .iter()
            .filter(|d| {
                d.enabled
                    && d.kind == DestinationKind::Telegram
                    && d.telegram_bot_token.as_deref().is_some_and(|v| !v.trim().is_empty())
                    && d.telegram_chat_id.as_deref().is_some_and(|v| !v.trim().is_empty())
            })
            .cloned()
            .collect()
    }

    async fn poll_destination(
        &mut self,
        dest: &NotificationDestination,
    ) -> Result<(), crate::error::NotificationError> {
        let offset = self.offsets.get(&dest.id).copied();
        let first_poll = !self.initialized.contains(&dest.id);
        let updates = telegram::get_updates(dest, offset, if first_poll { 0 } else { 20 }).await?;
        if updates.is_empty() {
            self.initialized.insert(dest.id.clone());
            return Ok(());
        }

        let next_offset = updates.iter().map(|u| u.update_id).max().unwrap_or(0) + 1;
        self.offsets.insert(dest.id.clone(), next_offset);

        if first_poll {
            self.initialized.insert(dest.id.clone());
            debug!("[telegram-commands] '{}' seeded offset {next_offset}", dest.label);
            return Ok(());
        }

        for update in updates {
            let Some(message) = update.message else { continue };
            if !chat_matches(dest, &message) || !thread_matches(dest, &message) {
                continue;
            }
            let Some(text) = message.text.as_deref().map(str::trim).filter(|v| !v.is_empty()) else {
                continue;
            };
            let cmd = normalize_command(text);
            match cmd.as_str() {
                "/status" => {
                    let body = self.status_text().await;
                    telegram::send_status(dest, "CC2 Status", &body).await?;
                    info!("[telegram-commands] status sent to '{}'", dest.label);
                }
                "/statusbild" | "/statuspic" | "/snapshot" | "/bild" | "/photo" => {
                    let body = self.status_text().await;
                    match self.frame_buffer.read().await.clone() {
                        Some(frame) => {
                            telegram::send_status_photo(dest, "CC2 Status", &body, frame).await?;
                        }
                        None => {
                            let body = format!("{body}\n\nCamera image is not available right now.");
                            telegram::send_status(dest, "CC2 Status", &body).await?;
                        }
                    }
                    info!("[telegram-commands] status image sent to '{}'", dest.label);
                }
                "/help" | "/start" => {
                    telegram::send_status(
                        dest,
                        "CC2 Telegram Commands",
                        "/status - printer status as text\n/statusbild - printer status with current camera image\n/bild - current status image",
                    ).await?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    async fn status_text(&self) -> String {
        let s = self.state.read().await;
        let phase = crate::printer::state::build_phase_info(
            s.full.machine_status.status,
            s.full.machine_status.sub_status,
            &s.full.print_status.state,
        );
        let filename = if s.full.print_status.filename.trim().is_empty() {
            "-"
        } else {
            s.full.print_status.filename.as_str()
        };
        let progress = s.full.machine_status.progress.clamp(0, 100);
        let nozzle = format!(
            "{:.0}/{:.0}C",
            s.full.extruder.temperature,
            s.full.extruder.target
        );
        let bed = format!(
            "{:.0}/{:.0}C",
            s.full.heater_bed.temperature,
            s.full.heater_bed.target
        );
        format!(
            "Printer: {}\nConnection: {} (raw {}, ws {})\nCamera: {}\nState: {}\nProgress: {}%\nFile: {}\nNozzle: {}\nBed: {}\nAI score: {:.0}%",
            if s.printer_ip.is_empty() { "-" } else { &s.printer_ip },
            if s.connected { "online" } else { "offline" },
            if s.connected_raw { "ok" } else { "off" },
            if s.connected_ws { "ok" } else { "off" },
            if s.camera_connected { "ok" } else { "off" },
            phase.label,
            progress,
            filename,
            nozzle,
            bed,
            s.detection_score * 100.0,
        )
    }
}

fn normalize_command(text: &str) -> String {
    let first = text.split_whitespace().next().unwrap_or("");
    let without_bot = first.split('@').next().unwrap_or(first);
    without_bot.to_ascii_lowercase()
}

fn chat_matches(dest: &NotificationDestination, message: &telegram::TelegramMessage) -> bool {
    dest.telegram_chat_id
        .as_deref()
        .map(|id| id.trim() == message.chat.id.to_string())
        .unwrap_or(false)
}

fn thread_matches(dest: &NotificationDestination, message: &telegram::TelegramMessage) -> bool {
    match dest.telegram_thread_id.as_deref().and_then(|v| v.trim().parse::<i64>().ok()) {
        Some(expected) => message.message_thread_id == Some(expected),
        None => true,
    }
}
