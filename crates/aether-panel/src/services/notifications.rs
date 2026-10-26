use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use super::runtime::runtime;
use aether::refresh;
use aether_services::notifications::{
    Notification, NotificationEvent, NotificationService, start_battery_alerts, start_wifi_alerts,
};
use aether_services::start_device_monitor;

// how many popups can sit on screen at once
pub const MAX_VISIBLE: usize = 3;

// how long a popup stays when the sender asks for the server default (0 or -1)
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

// how long the slide in or out takes
pub const SLIDE_TIME: Duration = Duration::from_millis(260);

// the part of the whole slide one frame covers, aimed at a 16ms frame
pub const STEP: f32 = 16.0 / SLIDE_TIME.as_millis() as f32;

// how long a card takes to brighten under the pointer and to settle back
pub const HOVER_TIME: Duration = Duration::from_millis(140);

// the part of the whole hover one frame covers
pub const HOVER_STEP: f32 = 16.0 / HOVER_TIME.as_millis() as f32;

// one popup on screen, with everything the view needs to slide and size it
struct Popup {
    notification: Notification,

    // 0 while shown, one popup width off to the right once slid out
    offset: f32,

    // true once the pointer has pinned it open, so the timeout leaves it alone
    pinned: bool,

    // true once it is on its way out, so it slides rather than being dropped
    leaving: bool,

    // the moment it slides out at
    expired_at: Instant,

    // where the pointer is over the card, and over its close button
    hovered: bool,
    hover: f32,
    close_hovered: bool,
    close_hover: f32,
}

#[derive(Debug, Clone)]
pub struct VisiblePopup {
    pub id: u32,
    pub notification: Notification,
    pub offset: f32,
    pub leaving: bool,

    // how bright the card is, and its close button, under the pointer
    pub hover: f32,
    pub close_hover: f32,
}

static POPUPS: OnceLock<Mutex<VecDeque<Popup>>> = OnceLock::new();
static HISTORY: OnceLock<Mutex<Vec<Notification>>> = OnceLock::new();
static DO_NOT_DISTURB: AtomicBool = AtomicBool::new(false);

// starts the listener exactly once
static STARTED: OnceLock<()> = OnceLock::new();

fn store() -> &'static Mutex<VecDeque<Popup>> {
    POPUPS.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn history() -> &'static Mutex<Vec<Notification>> {
    HISTORY.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn active_notifications() -> Vec<Notification> {
    start();
    history()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

pub fn do_not_disturb() -> bool {
    DO_NOT_DISTURB.load(Ordering::Relaxed)
}

pub fn toggle_do_not_disturb() {
    let enabled = !DO_NOT_DISTURB.load(Ordering::Relaxed);
    DO_NOT_DISTURB.store(enabled, Ordering::Relaxed);

    if enabled {
        store()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }

    refresh();
}

pub fn ensure_started() {
    start();
}

pub fn clear_all() {
    for notification in active_notifications() {
        close_notification(notification.id);
    }
}

pub fn close_notification(id: u32) {
    runtime().spawn(async move {
        if let Err(error) = aether_services::notifications::close_notification(id).await {
            eprintln!("aether-panel: could not close notification {id}: {error}");
        }
    });
}

// the popups to draw, oldest first, which is also top to bottom on screen
pub fn visible() -> Vec<VisiblePopup> {
    start();

    store()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .map(|popup| VisiblePopup {
            id: popup.notification.id,
            notification: popup.notification.clone(),
            offset: popup.offset,
            leaving: popup.leaving,
            hover: popup.hover,
            close_hover: popup.close_hover,
        })
        .collect()
}

/*
 * Stops the pointer pinning a popup open while it is over any popup,
 * so hovering holds the whole stack, matching how bars usually behave.
 */
pub fn set_hovered(hovered: bool) {
    let mut popups = store().lock().unwrap_or_else(PoisonError::into_inner);

    for popup in popups.iter_mut() {
        popup.pinned = hovered && !popup.leaving;
    }
}

/*
 * Remembers where the pointer is over a card, so the card can brighten under
 * it and the close button can light up on its own.
 */
pub fn set_card_hovered(id: u32, inside: bool) {
    let mut popups = store().lock().unwrap_or_else(PoisonError::into_inner);

    let Some(popup) = popups.iter_mut().find(|popup| popup.notification.id == id) else {
        return;
    };

    if popup.hovered == inside {
        return;
    }

    popup.hovered = inside;

    drop(popups);

    refresh();
}

// the same, for the close button of one card
pub fn set_close_hovered(id: u32, inside: bool) {
    let mut popups = store().lock().unwrap_or_else(PoisonError::into_inner);

    let Some(popup) = popups.iter_mut().find(|popup| popup.notification.id == id) else {
        return;
    };

    if popup.close_hovered == inside {
        return;
    }

    popup.close_hovered = inside;

    drop(popups);

    refresh();
}

// asks a popup to slide out now, like a click dismissing it
pub fn dismiss(id: u32) {
    let mut popups = store().lock().unwrap_or_else(PoisonError::into_inner);

    if let Some(popup) = popups.iter_mut().find(|popup| popup.notification.id == id) {
        popup.leaving = true;
        popup.expired_at = Instant::now();
        drop(popups);
        close_notification(id);
        refresh();
    }
}

fn start() {
    let mut ready_receiver = None;
    STARTED.get_or_init(|| {
        let (ready_sender, receiver) = std::sync::mpsc::sync_channel(1);
        ready_receiver = Some(receiver);
        runtime().spawn(async move {
            let service = loop {
                match NotificationService::start().await {
                    Ok(service) => {
                        let _ = ready_sender.send(());
                        break service;
                    }
                    Err(error) => {
                        eprintln!("aether: failed to start the notification service: {error}");
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                }
            };
            let mut events = service.subscribe();
            replace_history(service.active());

            // Start battery alerts only after the D-Bus notification server is ready.
            let _battery_monitor = start_battery_alerts();

            // Same for the wifi connect/disconnect notices.
            let _wifi_monitor = start_wifi_alerts();

            // And the usb/audio/display plug notices.
            let _device_monitor = start_device_monitor();

            // Animate popups while listening for notification events.
            runtime().spawn(animate());

            loop {
                match events.recv().await {
                    Ok(event) => {
                        if handle(event) {
                            refresh();
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        eprintln!("aether-panel: skipped {skipped} notification events");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    });
    if let Some(receiver) = ready_receiver
        && receiver.recv_timeout(Duration::from_secs(3)).is_err()
    {
        eprintln!("aether-panel: notification service did not become ready within 3 seconds");
    }
}

fn handle(event: NotificationEvent) -> bool {
    let history_changed = update_history(&event);

    if !should_enqueue_popup(do_not_disturb(), &event) {
        return history_changed;
    }

    let mut popups = store().lock().unwrap_or_else(PoisonError::into_inner);

    match event {
        NotificationEvent::Created(notification) => {
            // a replacement of a shown popup updates it in place, without sliding again
            if let Some(existing) = popups
                .iter_mut()
                .find(|popup| popup.notification.id == notification.id)
            {
                existing.notification = notification;

                return true;
            }

            let expired_at = expiry_of(&notification);

            popups.push_back(Popup {
                notification,
                offset: 1.0,
                pinned: false,
                leaving: false,
                expired_at,
                hovered: false,
                hover: 0.0,
                close_hovered: false,
                close_hover: 0.0,
            });

            while popups.len() > MAX_VISIBLE {
                popups.pop_front();
            }

            true
        }

        // the sender closed it: slide it out rather than dropping it at once
        NotificationEvent::Closed { id, .. } => {
            if let Some(popup) = popups.iter_mut().find(|popup| popup.notification.id == id) {
                popup.leaving = true;
                popup.expired_at = Instant::now();
            }

            history_changed
        }

        // an action is not shown yet, so it changes nothing on screen
        NotificationEvent::ActionInvoked { .. } => false,
    }
}

fn should_enqueue_popup(do_not_disturb: bool, event: &NotificationEvent) -> bool {
    !do_not_disturb || !matches!(event, NotificationEvent::Created(_))
}

fn update_history(event: &NotificationEvent) -> bool {
    let mut history = history().lock().unwrap_or_else(PoisonError::into_inner);
    let changed = match event {
        NotificationEvent::Created(notification) => {
            if let Some(existing) = history
                .iter_mut()
                .find(|existing| existing.id == notification.id)
            {
                if *existing == *notification {
                    false
                } else {
                    *existing = notification.clone();
                    true
                }
            } else {
                history.push(notification.clone());
                if history.len() > 100 {
                    history.remove(0);
                }
                true
            }
        }
        NotificationEvent::Closed { id, .. } => {
            let previous_len = history.len();
            history.retain(|item| item.id != *id);
            history.len() != previous_len
        }
        NotificationEvent::ActionInvoked { .. } => false,
    };
    drop(history);
    changed
}

fn replace_history(notifications: Vec<Notification>) {
    *history().lock().unwrap_or_else(PoisonError::into_inner) = notifications;
    refresh();
}

// the moment a popup slides out at, from what the sender asked for
fn expiry_of(notification: &Notification) -> Instant {
    Instant::now() + timeout_of(notification)
}

// the time a popup stays for, with 0 and -1 both meaning the server default
fn timeout_of(notification: &Notification) -> Duration {
    match notification.expire_timeout_ms {
        ms if ms <= 0 => DEFAULT_TIMEOUT,
        ms => Duration::from_millis(ms as u64),
    }
}

/*
 * Moves every popup towards its target, one step per frame at roughly
 * refresh rate, and drops the ones that slid out. Setting frames moving
 * keeps the window drawing until they arrive.
 */
async fn animate() {
    // a steady step, so a slide takes about SLIDE_TIME either way
    let step = STEP;

    // and one for the hover, so a card brightens in about HOVER_TIME
    let hover_step = HOVER_STEP;

    loop {
        tokio::time::sleep(Duration::from_millis(16)).await;

        let mut changed = false;

        {
            let mut popups = store().lock().unwrap_or_else(PoisonError::into_inner);

            for popup in popups.iter_mut() {
                // a shown popup that has run out of time starts leaving
                if !popup.leaving && !popup.pinned && Instant::now() >= popup.expired_at {
                    popup.leaving = true;
                }

                let next = advance(popup.offset, popup.leaving, step);

                if next != popup.offset {
                    popup.offset = next;

                    changed = true;
                }

                // the card brightens under the pointer and settles back when it leaves
                let hot = advance_hover(popup.hover, popup.hovered, hover_step);

                if hot != popup.hover {
                    popup.hover = hot;

                    changed = true;
                }

                let close = advance_hover(popup.close_hover, popup.close_hovered, hover_step);

                if close != popup.close_hover {
                    popup.close_hover = close;

                    changed = true;
                }
            }

            // anything fully slid out is gone
            let before = popups.len();

            popups.retain(|popup| !(popup.leaving && popup.offset >= 1.0));

            if popups.len() != before {
                changed = true;
            }
        }

        if changed {
            refresh();
        }
    }
}

// the offset one frame later: towards flush once shown, off the edge once leaving
fn advance(offset: f32, leaving: bool, step: f32) -> f32 {
    if leaving {
        f32::min(offset + step, 1.0)
    } else {
        f32::max(offset - step, 0.0)
    }
}

// the hover one frame later: towards full under the pointer, back to none without it
fn advance_hover(hover: f32, inside: bool, step: f32) -> f32 {
    if inside {
        f32::min(hover + step, 1.0)
    } else {
        f32::max(hover - step, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn notification(expire: i32) -> Notification {
        Notification::new(
            1,
            "App".into(),
            String::new(),
            String::new(),
            "Summary".into(),
            "Body".into(),
            Vec::new(),
            expire,
        )
    }

    #[test]
    fn a_zero_timeout_uses_the_default() {
        assert_eq!(timeout_of(&notification(0)), DEFAULT_TIMEOUT);
    }

    #[test]
    fn a_negative_timeout_also_uses_the_default() {
        // notify-send with no -t sends -1, and that has to close like any other
        assert_eq!(timeout_of(&notification(-1)), DEFAULT_TIMEOUT);
    }

    #[test]
    fn do_not_disturb_keeps_notifications_out_of_the_popup_queue() {
        let created = NotificationEvent::Created(notification(0));
        let closed = NotificationEvent::Closed { id: 1, reason: 3 };

        assert!(!should_enqueue_popup(true, &created));
        assert!(should_enqueue_popup(false, &created));
        assert!(should_enqueue_popup(true, &closed));
    }

    #[test]
    fn a_positive_timeout_is_taken_as_given() {
        assert_eq!(timeout_of(&notification(2500)), Duration::from_millis(2500));
    }

    #[test]
    fn every_popup_gets_a_moment_to_slide_out_at() {
        // a popup that never expires is what left the old one stuck on screen
        let expiry = expiry_of(&notification(-1));

        assert!(expiry > Instant::now());
        assert!(expiry <= Instant::now() + DEFAULT_TIMEOUT);
    }

    #[test]
    fn sliding_in_stops_flush_and_out_stops_at_the_edge() {
        let step = STEP;

        assert_eq!(advance(1.0, false, step), 1.0 - step);
        assert_eq!(advance(0.0, false, step), 0.0);
        assert_eq!(advance(0.9, true, 0.2), 1.0);
        assert_eq!(advance(1.0, true, step), 1.0);
    }

    #[test]
    fn a_slide_covers_the_whole_way_in_about_slide_time() {
        // one step per 16ms frame should cross in roughly SLIDE_TIME
        let frames = (1.0 / STEP).ceil();
        let taken = frames * 16.0;

        assert!(taken <= SLIDE_TIME.as_millis() as f32 + 16.0);
    }
}
