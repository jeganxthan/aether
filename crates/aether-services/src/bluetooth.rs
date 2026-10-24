use std::io;
use std::process::Command;

const COMMAND_TIMEOUT: &str = "3s";

pub fn powered() -> io::Result<bool> {
    let output = Command::new("timeout")
        .args([COMMAND_TIMEOUT, "bluetoothctl", "show"])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }

    parse_powered(&String::from_utf8_lossy(&output.stdout))
        .ok_or_else(|| io::Error::other("Bluetooth controller power state is missing"))
}

pub fn set_power(enabled: bool) -> io::Result<bool> {
    let output = Command::new("timeout")
        .args([
            COMMAND_TIMEOUT,
            "bluetoothctl",
            "power",
            if enabled { "on" } else { "off" },
        ])
        .output()?;
    if !output.status.success() {
        return Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }

    let actual = powered()?;
    if actual != enabled {
        return Err(io::Error::other(format!(
            "Bluetooth remained {} after requesting {}",
            if actual { "powered on" } else { "powered off" },
            if enabled { "power on" } else { "power off" },
        )));
    }

    Ok(actual)
}

pub fn toggle_power() -> io::Result<bool> {
    set_power(!powered()?)
}

fn parse_powered(output: &str) -> Option<bool> {
    output.lines().find_map(|line| {
        line.trim()
            .strip_prefix("Powered:")
            .map(|value| value.trim() == "yes")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_powered_state_without_matching_powerstate() {
        assert_eq!(
            parse_powered("Controller X\n\tPowered: yes\n\tPowerState: on\n"),
            Some(true)
        );
        assert_eq!(parse_powered("\tPowered: no\n"), Some(false));
        assert_eq!(parse_powered("Controller has no power field\n"), None);
    }
}
