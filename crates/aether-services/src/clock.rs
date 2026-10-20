use chrono::Local;
use tokio::sync::watch;
use tokio::time::{interval, MissedTickBehavior, Duration};

#[derive(Debug, Clone)]
pub struct ClockService {
    receiver: watch::Receiver<String>,
}

impl ClockService {
    pub fn new() -> Self {
        let (sender, receiver) = watch::channel(current_time());

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(1));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                ticker.tick().await;
                let _ = sender.send(current_time());
            }
        });

        Self { receiver }
    }

    pub fn current(&self) -> String {
        self.receiver.borrow().clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<String> {
        self.receiver.clone()
    }
}

impl Default for ClockService {
    fn default() -> Self {
        Self::new()
    }
}

fn current_time() -> String {
    Local::now().format("%H:%M").to_string()
}