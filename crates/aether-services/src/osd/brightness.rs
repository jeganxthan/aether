
use std::{
    fs,
    io,
    path::PathBuf,
    process::Command,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrightnessState {
    pub percent: u8,
    pub device: String,
}

pub struct BrightnessService;

impl BrightnessService {
    pub fn current() -> io::Result<BrightnessState> {
        let root = PathBuf::from("/sys/class/backlight");

        for entry in fs::read_dir(&root)? {
            let entry = entry?;
            let path = entry.path();

            let Ok(current) = fs::read_to_string(path.join("brightness")) else {
                continue;
            };
            let Ok(maximum) = fs::read_to_string(path.join("max_brightness")) else {
                continue;
            };

            let current: u64 = current.trim().parse().map_err(io::Error::other)?;
            let maximum: u64 = maximum.trim().parse().map_err(io::Error::other)?;

            if maximum == 0 {
                continue;
            }

            return Ok(BrightnessState {
                percent: ((current as f64 / maximum as f64) * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u8,
                device: entry.file_name().to_string_lossy().into_owned(),
            });
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No readable backlight device found",
        ))
    }

    pub fn increase() -> io::Result<BrightnessState> {
        Self::adjust("+5%")
    }

    pub fn decrease() -> io::Result<BrightnessState> {
        Self::adjust("5%-")
    }

    fn adjust(amount: &str) -> io::Result<BrightnessState> {
        let output = Command::new("brightnessctl")
            .args(["set", amount])
            .output()?;

        if !output.status.success() {
            return Err(io::Error::other(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ));
        }

        Self::current()
    }
}
