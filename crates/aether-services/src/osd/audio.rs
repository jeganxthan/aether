
use std::{
    io,
    process::Command,
};

const AUDIO_SINK: &str = "@DEFAULT_AUDIO_SINK@";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioState {
    pub volume_percent: u8,
    pub muted: bool,
}

pub struct AudioService;

impl AudioService {
    pub fn current() -> io::Result<AudioState> {
        let output = Command::new("wpctl")
            .args(["get-volume", AUDIO_SINK])
            .output()?;

        if !output.status.success() {
            return Err(io::Error::other(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ));
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let volume = text
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| io::Error::other("Unexpected wpctl output"))?
            .parse::<f32>()
            .map_err(io::Error::other)?;

        Ok(AudioState {
            volume_percent: (volume * 100.0).round().clamp(0.0, 100.0) as u8,
            muted: text.contains("[MUTED]"),
        })
    }

    pub fn increase() -> io::Result<AudioState> {
        Self::run(&["set-volume", "-l", "1.0", AUDIO_SINK, "5%+"])?;
        Self::current()
    }

    pub fn decrease() -> io::Result<AudioState> {
        Self::run(&["set-volume", AUDIO_SINK, "5%-"])?;
        Self::current()
    }

    pub fn toggle_mute() -> io::Result<AudioState> {
        Self::run(&["set-mute", AUDIO_SINK, "toggle"])?;
        Self::current()
    }

    fn run(args: &[&str]) -> io::Result<()> {
        let output = Command::new("wpctl").args(args).output()?;

        if output.status.success() {
            Ok(())
        } else {
            Err(io::Error::other(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ))
        }
    }
}
