
use serde::Serialize;
use std::{io, process::Command};

const AUDIO_SINK: &str = "@DEFAULT_AUDIO_SINK@";
const AUDIO_SOURCE: &str = "@DEFAULT_AUDIO_SOURCE@";

// ─── State types ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MicrophoneState {
    pub muted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioState {
    pub volume_percent: u8,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub name: String,
    pub description: String,
    pub is_default: bool,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevices {
    pub outputs: Vec<AudioDevice>,
    pub inputs: Vec<AudioDevice>,
    pub default_output: Option<String>,
    pub default_input: Option<String>,
}

// ─── Audio output service ────────────────────────────────────

pub struct AudioService;

impl AudioService {
    pub fn current() -> io::Result<AudioState> {
        let text = get_volume(AUDIO_SINK)?;
        let volume = parse_volume(&text)?;

        Ok(AudioState {
            volume_percent: (volume * 100.0).round().clamp(0.0, 100.0) as u8,
            muted: parse_muted(&text),
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

    pub fn set_volume(percent: u8) -> io::Result<AudioState> {
        let percent = percent.min(100).to_string();

        Self::run(&[
            "set-volume",
            "-l",
            "1.0",
            AUDIO_SINK,
            &format!("{percent}%"),
        ])?;

        Self::current()
    }

    fn run(args: &[&str]) -> io::Result<()> {
        run_command("wpctl", args).map(|_| ())
    }
}

// ─── Microphone service ──────────────────────────────────────

pub struct MicrophoneService;

impl MicrophoneService {
    pub fn current() -> io::Result<MicrophoneState> {
        let text = get_volume(AUDIO_SOURCE)?;

        Ok(MicrophoneState {
            muted: parse_muted(&text),
        })
    }

    pub fn toggle_mute() -> io::Result<MicrophoneState> {
        run_command("wpctl", &["set-mute", AUDIO_SOURCE, "toggle"])?;
        Self::current()
    }

    pub fn set_mute(muted: bool) -> io::Result<MicrophoneState> {
        run_command(
            "wpctl",
            &[
                "set-mute",
                AUDIO_SOURCE,
                if muted { "1" } else { "0" },
            ],
        )?;

        Self::current()
    }
}

// ─── Device discovery and selection ───────────────────────────

impl AudioService {
    fn pactl(args: &[&str]) -> io::Result<String> {
        run_command("pactl", args)
    }

    fn default_device(command: &str) -> io::Result<Option<String>> {
        let value = Self::pactl(&[command])?;

        Ok(if value.is_empty() {
            None
        } else {
            Some(value)
        })
    }

    fn list_devices(
        kind: &str,
        default_name: Option<&str>,
    ) -> io::Result<Vec<AudioDevice>> {
        let output = Self::pactl(&["list", "short", kind])?;

        let mute_command = if kind == "sources" {
            "get-source-mute"
        } else {
            "get-sink-mute"
        };

        let mut devices = Vec::new();

        for line in output.lines() {
            let mut fields = line.split_whitespace();

            // pactl list short: index, name, driver, sample specification...
            let Some(_index) = fields.next() else {
                continue;
            };

            let Some(name) = fields.next() else {
                continue;
            };

            let name = name.to_owned();

            // Monitor sources record output audio, not a physical microphone.
            if kind == "sources" && name.ends_with(".monitor") {
                continue;
            }

            let muted_output = Self::pactl(&[mute_command, &name])?;
            let muted = muted_output
                .split_whitespace()
                .last()
                .is_some_and(|value| value == "yes");

            devices.push(AudioDevice {
                description: name.clone(),
                is_default: default_name == Some(name.as_str()),
                name,
                muted,
            });
        }

        Ok(devices)
    }

    pub fn devices() -> io::Result<AudioDevices> {
        let default_output = Self::default_device("get-default-sink")?;
        let default_input = Self::default_device("get-default-source")?;

        let outputs =
            Self::list_devices("sinks", default_output.as_deref())?;

        let inputs =
            Self::list_devices("sources", default_input.as_deref())?;

        Ok(AudioDevices {
            outputs,
            inputs,
            default_output,
            default_input,
        })
    }

    pub fn select_output(name: &str) -> io::Result<AudioDevices> {
        Self::validate_device_name(name)?;

        Self::pactl(&["set-default-sink", name])?;
        Self::devices()
    }

    pub fn select_input(name: &str) -> io::Result<AudioDevices> {
        Self::validate_device_name(name)?;

        Self::pactl(&["set-default-source", name])?;
        Self::devices()
    }

    pub fn set_microphone_mute(
        muted: bool,
    ) -> io::Result<AudioDevices> {
        let source = Self::default_device("get-default-source")?
            .ok_or_else(|| io::Error::other("No default microphone found"))?;

        Self::pactl(&[
            "set-source-mute",
            &source,
            if muted { "1" } else { "0" },
        ])?;

        Self::devices()
    }

    fn validate_device_name(name: &str) -> io::Result<()> {
        if name.is_empty()
            || name.chars().any(char::is_whitespace)
            || name.starts_with('-')
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid audio device name",
            ));
        }

        Ok(())
    }
}

// ─── Shared command and parsing helpers ──────────────────────

fn run_command(program: &str, args: &[&str]) -> io::Result<String> {
    let output = Command::new(program).args(args).output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr)
            .trim()
            .to_owned();

        return Err(io::Error::other(if error.is_empty() {
            format!("{program} command failed")
        } else {
            error
        }));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn get_volume(device: &str) -> io::Result<String> {
    run_command("wpctl", &["get-volume", device])
}

fn parse_volume(text: &str) -> io::Result<f32> {
    text.split_whitespace()
        .nth(1)
        .ok_or_else(|| io::Error::other("Unexpected wpctl output"))?
        .parse::<f32>()
        .map_err(io::Error::other)
}

fn parse_muted(text: &str) -> bool {
    text.contains("[MUTED]")
}
