use chrono::Local;
use tokio::sync::watch;
use tokio::time::{interval, MissedTickBehavior, Duration};

#[derive(Debug, Clone)]
pub struct ClockService {
    receiver: watch::Receiver<String>,
    date: watch::Receiver<String>,
}

impl ClockService {
    pub fn new() -> Self {
        let (sender, receiver) = watch::channel(current_time());
        let (date_sender, date) = watch::channel(current_date());

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(1));
            ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

            loop {
                ticker.tick().await;
                let _ = sender.send(current_time());
                let _ = date_sender.send(current_date());
            }
        });

        Self { receiver, date }
    }

    pub fn current(&self) -> String {
        self.receiver.borrow().clone()
    }

    // the month and day, like OCT 18
    pub fn today(&self) -> String {
        self.date.borrow().clone()
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

// the month and day, like OCT 18, for a bar that shows both
fn current_date() -> String {
    Local::now().format("%b %d").to_string().to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_date_is_an_upper_month_and_day() {
        let today = current_date();

        let mut parts = today.split(' ');

        let month = parts.next().expect("a month");
        let day = parts.next().expect("a day");

        assert!(parts.next().is_none(), "the date has extra parts: {today}");

        // three letters, all upper case, like OCT
        assert_eq!(month.len(), 3, "unexpected month: {month}");
        assert!(month.chars().all(|c| c.is_ascii_uppercase()), "the month is not upper: {month}");

        // a plain day number, like 18
        assert!(day.parse::<u32>().is_ok(), "unexpected day: {day}");
    }

    #[test]
    fn the_time_is_hours_and_minutes() {
        let time = current_time();

        let mut parts = time.split(':');

        let hours = parts.next().expect("hours");
        let minutes = parts.next().expect("minutes");

        assert!(parts.next().is_none(), "the time has extra parts: {time}");
        assert_eq!(hours.len(), 2, "unexpected hours: {hours}");
        assert_eq!(minutes.len(), 2, "unexpected minutes: {minutes}");
    }
}