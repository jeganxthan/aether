use crate::notifications::client;

/*
 * Shows a notification in the shell's own popup, through the same server the
 * panel runs. Failures are ignored: a missing notifier must never break the
 * action that produced it.
 */
pub fn notify(app_name: &str, app_icon: &str, summary: &str, body: &str) {
    let _ = client::publish_now(app_name, app_icon, summary, body);
}
