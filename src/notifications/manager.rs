use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::{broadcast, RwLock};
use tracing::{info, warn};

use crate::config::{AppConfig, DestinationKind, EventToggles, NotificationDestination};
use crate::printer::state::{EventKind, PrinterEvent, PrinterState};

use super::{discord, ntfy, payload, telegram, webhook};

const COOLDOWN_SECS: u64 = 120;
const CONNECTION_EVENT_DELAY_SECS: u64 = 60;

struct PendingConnectionEvent {
    event: PrinterEvent,
    due_at: Instant,
}

pub struct NotificationManager {
    state: Arc<RwLock<PrinterState>>,
    config: Arc<RwLock<AppConfig>>,
    /// last events_total
    last_processed_total: u64,
    cooldowns: HashMap<String, Instant>,
    pending_connection: Option<PendingConnectionEvent>,
    connection_outage_notified: bool,
}

impl NotificationManager {
    pub fn new(state: Arc<RwLock<PrinterState>>, config: Arc<RwLock<AppConfig>>) -> Self {
        // seed current total
        let last_processed_total = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                state.read().await.events_total
            })
        });
        Self {
            state,
            config,
            last_processed_total,
            cooldowns: HashMap::new(),
            pending_connection: None,
            connection_outage_notified: false,
        }
    }

    pub async fn run(mut self, mut state_changed_rx: broadcast::Receiver<()>) {
        loop {
            if let Some(delay) = self.connection_delay_until_due() {
                tokio::select! {
                    res = state_changed_rx.recv() => {
                        match res {
                            Ok(()) => self.process_new_events().await,
                            Err(broadcast::error::RecvError::Lagged(n)) => {
                                warn!("[notifications] missed {n} state updates");
                                self.process_new_events().await;
                            }
                            Err(broadcast::error::RecvError::Closed) => return,
                        }
                    }
                    _ = tokio::time::sleep(delay) => {
                        self.dispatch_due_connection_event().await;
                    }
                }
            } else {
                match state_changed_rx.recv().await {
                    Ok(()) => self.process_new_events().await,
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("[notifications] missed {n} state updates");
                        self.process_new_events().await;
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        }
    }

    fn connection_delay_until_due(&self) -> Option<Duration> {
        self.pending_connection
            .as_ref()
            .map(|pending| pending.due_at.saturating_duration_since(Instant::now()))
    }

    async fn process_new_events(&mut self) {
        let (new_events, destinations) = {
            let state = self.state.read().await;
            let events_total = state.events_total;
            let events = &state.events;

            // new events count
            let unprocessed = (events_total.saturating_sub(self.last_processed_total)) as usize;
            // cap by buffer
            let to_take = unprocessed.min(events.len());
            let new: Vec<PrinterEvent> = events[events.len() - to_take..].to_vec();

            self.last_processed_total = events_total;

            let destinations = self.config.read().await.notifications.destinations.clone();
            (new, destinations)
        };

        for event in &new_events {
            if self.queue_connection_event(event) {
                continue;
            }

            self.dispatch_event_to_destinations(event, &destinations).await;
        }
    }

    fn queue_connection_event(&mut self, event: &PrinterEvent) -> bool {
        match event.kind {
            EventKind::Disconnected => {
                self.pending_connection = Some(PendingConnectionEvent {
                    event: event.clone(),
                    due_at: Instant::now() + Duration::from_secs(CONNECTION_EVENT_DELAY_SECS),
                });
                true
            }
            EventKind::Connected => {
                if matches!(
                    self.pending_connection.as_ref().map(|pending| &pending.event.kind),
                    Some(EventKind::Disconnected)
                ) {
                    self.pending_connection = None;
                    return true;
                }

                if self.connection_outage_notified {
                    self.pending_connection = Some(PendingConnectionEvent {
                        event: event.clone(),
                        due_at: Instant::now() + Duration::from_secs(CONNECTION_EVENT_DELAY_SECS),
                    });
                }
                true
            }
            _ => false,
        }
    }

    async fn dispatch_due_connection_event(&mut self) {
        let Some(pending) = self.pending_connection.take() else {
            return;
        };

        let (connected, destinations) = {
            let state = self.state.read().await;
            let connected = state.connected;
            let destinations = self.config.read().await.notifications.destinations.clone();
            (connected, destinations)
        };

        match pending.event.kind {
            EventKind::Disconnected if !connected => {
                self.dispatch_event_to_destinations(&pending.event, &destinations).await;
                self.connection_outage_notified = true;
            }
            EventKind::Connected if connected => {
                self.dispatch_event_to_destinations(&pending.event, &destinations).await;
                self.connection_outage_notified = false;
            }
            _ => {}
        }
    }

    async fn dispatch_event_to_destinations(
        &mut self,
        event: &PrinterEvent,
        destinations: &[NotificationDestination],
    ) {
        for dest in destinations {
            if !dest.enabled {
                continue;
            }
            if !event_matches_toggles(&event.kind, &dest.toggles) {
                continue;
            }
            if !event_matches_destination_options(&event.kind, dest) {
                continue;
            }

            let key = format!("{}:{}", dest.id, cooldown_label(&event.kind));
            if let Some(last) = self.cooldowns.get(&key) {
                if last.elapsed() < Duration::from_secs(COOLDOWN_SECS) {
                    continue;
                }
            }
            self.cooldowns.insert(key, Instant::now());

            let p = payload::format_event(event);
            dispatch(dest, event, &p.title, &p.body, p.color).await;
        }
    }
}

fn cooldown_label(kind: &EventKind) -> String {
    match kind {
        EventKind::ProgressMilestone(percent) => format!("progress_milestone_{percent}"),
        _ => event_kind_label(kind).to_string(),
    }
}

fn event_kind_label(kind: &EventKind) -> &'static str {
    match kind {
        EventKind::PrintStarted => "print_started",
        EventKind::PrintFinished => "print_finished_ok",
        EventKind::PrintPaused => "print_paused",
        EventKind::PrintResumed => "print_resumed",
        EventKind::PrintStopped => "print_stopped",
        EventKind::ProgressMilestone(_) => "progress_milestone",
        EventKind::FailureNotifyThreshold => "failure_notify",
        EventKind::FailurePauseThreshold => "failure_pause",
        EventKind::AutoPaused => "auto_paused",
        EventKind::CameraLost => "camera_lost",
        EventKind::CameraRestored => "camera_restored",
        EventKind::Connected => "connected",
        EventKind::Disconnected => "disconnected",
        EventKind::PhaseChanged(code, _) => match code {
            19 => "emergency_stop",
            999 => "machine_error",
            1000 => "id_not_match",
            1001 => "auth_error",
            _ => "other",
        },
        EventKind::DetectionEngineError => "detection_engine_error",
        _ => "other",
    }
}

fn event_matches_destination_options(kind: &EventKind, dest: &NotificationDestination) -> bool {
    match kind {
        EventKind::ProgressMilestone(percent) => {
            let interval = dest.progress_interval;
            interval > 0 && *percent > 0 && *percent % interval == 0
        }
        _ => true,
    }
}

fn event_matches_toggles(kind: &EventKind, t: &EventToggles) -> bool {
    match kind {
        EventKind::PrintStarted => t.print_started,
        EventKind::PrintFinished => t.print_finished_ok,
        EventKind::PrintPaused => t.print_paused,
        EventKind::PrintResumed => t.print_resumed,
        EventKind::PrintStopped => t.print_stopped,
        EventKind::ProgressMilestone(_) => t.progress_milestone,
        EventKind::FailureNotifyThreshold => t.failure_notify,
        EventKind::FailurePauseThreshold => t.failure_pause,
        EventKind::AutoPaused => t.auto_paused,
        EventKind::CameraLost => t.camera_lost,
        EventKind::CameraRestored => t.camera_restored,
        EventKind::Connected => t.connected,
        EventKind::Disconnected => t.disconnected,
        EventKind::PhaseChanged(code, _) => match code {
            19 => t.emergency_stop,
            999 => t.machine_error,
            1000 => t.id_not_match,
            1001 => t.auth_error,
            _ => false,
        },
        EventKind::DetectionEngineError => t.detection_engine_error,
        _ => false,
    }
}

async fn dispatch(dest: &NotificationDestination, event: &PrinterEvent, title: &str, body: &str, color: u32) {
    let snapshot_path = event
        .snapshot
        .as_deref()
        .filter(|_| dest.attach_snapshot)
        .map(|filename| std::path::Path::new("snapshots").join(filename));

    match dest.kind {
        DestinationKind::Ntfy => match ntfy::send(dest, title, body).await {
            Ok(()) => info!("[notifications] ntfy '{}' sent: {title}", dest.label),
            Err(e) => warn!("[notifications] ntfy '{}' failed: {e}", dest.label),
        },
        DestinationKind::Discord => match discord::send(dest, title, body, color).await {
            Ok(()) => info!("[notifications] discord '{}' sent: {title}", dest.label),
            Err(e) => warn!("[notifications] discord '{}' failed: {e}", dest.label),
        },
        DestinationKind::Telegram => match telegram::send(dest, title, body, snapshot_path.as_deref()).await {
            Ok(()) => info!("[notifications] telegram '{}' sent: {title}", dest.label),
            Err(e) => warn!("[notifications] telegram '{}' failed: {e}", dest.label),
        },
        DestinationKind::Webhook => match webhook::send(dest, title, body).await {
            Ok(()) => info!("[notifications] webhook '{}' sent: {title}", dest.label),
            Err(e) => warn!("[notifications] webhook '{}' failed: {e}", dest.label),
        },
    }
}
