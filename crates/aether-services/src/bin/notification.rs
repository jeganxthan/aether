
use aether_services::notifications::{NotificationEvent, NotificationService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = NotificationService::start().await?;

    println!("Aether notification service is running.");
    println!("Listening on org.freedesktop.Notifications");

    let mut events = service.subscribe();

    loop {
        match events.recv().await {
            Ok(NotificationEvent::Created(notification)) => {
                println!("\n--- Notification received ---");
                println!("ID:      {}", notification.id);
                println!("App:     {}", notification.app_name);
                println!("Summary: {}", notification.summary);
                println!("Body:    {}", notification.body);
                println!("Timeout: {} ms", notification.expire_timeout_ms);
            }

            Ok(NotificationEvent::Closed { id, reason }) => {
                println!("Notification {id} closed (reason {reason})");
            }

            Ok(NotificationEvent::ActionInvoked { id, action_key }) => {
                println!("Action '{action_key}' invoked for notification {id}");
            }

            Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                eprintln!("Missed {count} notification events");
            }

            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
        }
    }

    Ok(())
}
