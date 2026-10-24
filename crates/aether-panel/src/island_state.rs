use aether::Service;
use std::time::Duration;

pub struct IslandState {
    pub expanded: bool,
}

impl Service for IslandState {
    fn new() -> Self {
        Self { expanded: false }
    }

    fn interval() -> Duration {
        Duration::from_secs(3600)
    }

    fn listen() {}
}
