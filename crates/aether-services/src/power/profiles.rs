use serde::Serialize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum PowerProfile {
    #[serde(rename = "power-saver")]
    Saver,

    #[serde(rename = "balanced")]
    Balanced,

    #[serde(rename = "performance")]
    Performance,
}

impl PowerProfile {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Saver => "power-saver",
            Self::Balanced => "balanced",
            Self::Performance => "performance",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Saver => "Power Saver",
            Self::Balanced => "Balanced",
            Self::Performance => "Performance",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "power-saver" => Some(Self::Saver),
            "balanced" => Some(Self::Balanced),
            "performance" => Some(Self::Performance),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PowerProfileChange {
    pub previous: Option<PowerProfile>,
    pub active: PowerProfile,
}

#[derive(Debug)]
pub enum PowerProfileError {
    CommandNotFound,
    SysfsUnavailable(String),
    DaemonUnavailable(String),
    ProfileUnavailable(String),
    CommandFailed(String),
    VerificationFailed { requested: String, actual: String },
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerProfileStatus {
    pub active: Option<PowerProfile>,
    pub available: Vec<PowerProfile>,
    pub supported: bool,
    pub backend: ProfileBackend,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProfileBackend {
    PowerProfilesDaemon,
    CpuFreq,
    FirmwarePlatformProfile,
    Unavailable,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuPolicyStatus {
    pub policy: String,
    pub governors: Vec<String>,
    pub current_governor: Option<String>,
    pub epp_preferences: Vec<String>,
    pub current_epp: Option<String>,
}
impl std::fmt::Display for PowerProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommandNotFound => {
                write!(f, "powerprofilesctl is not installed")
            }
            Self::SysfsUnavailable(message) => {
                write!(f, "System power profiles are unavailable: {message}")
            }
            Self::DaemonUnavailable(message) => {
                write!(f, "Power profile service unavailable: {message}")
            }
            Self::ProfileUnavailable(profile) => {
                write!(f, "Power profile is not available: {profile}")
            }
            Self::CommandFailed(message) => {
                write!(f, "Failed to change power profile: {message}")
            }
            Self::VerificationFailed { requested, actual } => {
                write!(f, "Requested {requested}, but active profile is {actual}")
            }
        }
    }
}

impl std::error::Error for PowerProfileError {}

fn write_sysfs(path: &Path, value: &str) -> Result<(), PowerProfileError> {
    match fs::write(path, value) {
        Ok(()) => return Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {}
        Err(error) => {
            return Err(PowerProfileError::SysfsUnavailable(format!(
                "cannot write {}: {error}",
                path.display()
            )));
        }
    }

    let platform_profile = Path::new("/sys/firmware/acpi/platform_profile");
    if path != platform_profile {
        return Err(PowerProfileError::SysfsUnavailable(format!(
            "passwordless profile changes are only configured for {}",
            platform_profile.display()
        )));
    }

    let tee = ["/usr/bin/tee", "/bin/tee"]
        .into_iter()
        .find(|path| Path::new(path).is_file())
        .ok_or_else(|| {
            PowerProfileError::SysfsUnavailable("tee is unavailable for the profile change".into())
        })?;
    let mut child = Command::new("pkexec")
        .arg(tee)
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            PowerProfileError::SysfsUnavailable(format!(
                "cannot request Polkit authorization with pkexec: {error}"
            ))
        })?;

    child
        .stdin
        .take()
        .ok_or_else(|| {
            PowerProfileError::SysfsUnavailable(
                "could not pass the selected profile to Polkit".into(),
            )
        })?
        .write_all(value.as_bytes())
        .map_err(|error| {
            PowerProfileError::SysfsUnavailable(format!(
                "could not pass the selected profile to Polkit: {error}"
            ))
        })?;

    let output = child.wait_with_output().map_err(|error| {
        PowerProfileError::SysfsUnavailable(format!(
            "could not wait for Polkit authorization: {error}"
        ))
    })?;

    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(PowerProfileError::SysfsUnavailable(if message.is_empty() {
            "Polkit authorization was cancelled or denied".into()
        } else {
            format!("Polkit profile change failed: {message}")
        }));
    }

    Ok(())
}

pub struct PowerProfileService;

impl PowerProfileService {
    pub fn new() -> Self {
        Self
    }

    fn run(args: &[&str]) -> Result<String, PowerProfileError> {
        let output = Command::new("powerprofilesctl")
            .args(args)
            .output()
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    PowerProfileError::CommandNotFound
                } else {
                    PowerProfileError::DaemonUnavailable(error.to_string())
                }
            })?;

        if !output.status.success() {
            return Err(PowerProfileError::DaemonUnavailable(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    }

    pub fn active_profile(&self) -> Result<PowerProfile, PowerProfileError> {
        if let Ok(profile) = Self::sysfs_active_profile() {
            return Ok(profile);
        }

        let output = match Self::run(&["get"]) {
            Ok(output) => output,
            Err(PowerProfileError::CommandNotFound) => {
                return Self::sysfs_active_profile();
            }
            Err(error) => return Err(error),
        };

        PowerProfile::parse(&output).ok_or_else(|| {
            PowerProfileError::CommandFailed(format!("Unknown active profile returned: {output}"))
        })
    }

    pub fn available_profiles(&self) -> Result<Vec<PowerProfile>, PowerProfileError> {
        let mut available = Self::sysfs_available_profiles().unwrap_or_default();
        let output = match Self::run(&["list"]) {
            Ok(output) => output,
            Err(PowerProfileError::CommandNotFound) => {
                return Ok(available);
            }
            Err(_) if !available.is_empty() => {
                return Ok(available);
            }
            Err(error) => return Err(error),
        };

        // powerprofilesctl list describes each profile in its output.
        // Match profile names as complete profile-header lines.
        let daemon_available = output
            .lines()
            .filter_map(|line| {
                let line = line.trim().trim_start_matches('*').trim();
                let name = line.strip_suffix(':')?;
                PowerProfile::parse(name)
            })
            .collect::<Vec<_>>();

        for profile in daemon_available {
            if !available.contains(&profile) {
                available.push(profile);
            }
        }

        Ok(available)
    }

    pub fn set_profile(
        &self,
        requested: PowerProfile,
    ) -> Result<PowerProfileChange, PowerProfileError> {
        let previous = self.active_profile().ok();

        let available = self.available_profiles()?;

        if !available.contains(&requested) {
            return Err(PowerProfileError::ProfileUnavailable(
                requested.as_str().to_owned(),
            ));
        }

        // Don't report a change if the requested profile is already active.
        if previous == Some(requested) {
            return Ok(PowerProfileChange {
                previous,
                active: requested,
            });
        }

        if PlatformProfile::read().is_ok_and(|platform| platform.supports(requested)) {
            return Self::sysfs_set_profile(requested, previous);
        }

        match Self::run(&["set", requested.as_str()]) {
            Ok(_) => {}
            Err(PowerProfileError::CommandNotFound) => {
                return Self::sysfs_set_profile(requested, previous);
            }
            Err(error) => {
                return Err(PowerProfileError::CommandFailed(error.to_string()));
            }
        }

        // Verify the actual state instead of trusting the command alone.
        let actual = self.active_profile()?;

        if actual != requested {
            return Err(PowerProfileError::VerificationFailed {
                requested: requested.as_str().to_owned(),
                actual: actual.as_str().to_owned(),
            });
        }

        Ok(PowerProfileChange {
            previous,
            active: actual,
        })
    }
    pub fn profile_status(&self) -> PowerProfileStatus {
        let active = self.active_profile().ok();
        let available = self.available_profiles().unwrap_or_default();

        let backend = if Self::run(&["get"]).is_ok() {
            ProfileBackend::PowerProfilesDaemon
        } else if PlatformProfile::read().is_ok() {
            ProfileBackend::FirmwarePlatformProfile
        } else if Self::sysfs_policies().is_ok() {
            ProfileBackend::CpuFreq
        } else {
            ProfileBackend::Unavailable
        };

        let supported = !available.is_empty() && active.is_some();

        PowerProfileStatus {
            active,
            available,
            supported,
            backend,
            detail: if active.is_none() {
                Some("Could not determine the active power profile".into())
            } else {
                None
            },
        }
    }
    pub fn cpu_policy_status(&self) -> Result<Vec<CpuPolicyStatus>, PowerProfileError> {
        let policies = Self::sysfs_policies()?;

        Ok(policies
            .into_iter()
            .map(|policy| {
                let current_governor = fs::read_to_string(policy.path.join("scaling_governor"))
                    .ok()
                    .map(|value| value.trim().to_owned());

                let current_epp =
                    fs::read_to_string(policy.path.join("energy_performance_preference"))
                        .ok()
                        .map(|value| value.trim().to_owned());

                CpuPolicyStatus {
                    policy: policy
                        .path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                    governors: policy.governors,
                    current_governor,
                    epp_preferences: policy.epp_preferences,
                    current_epp,
                }
            })
            .collect())
    }
    fn sysfs_policies() -> Result<Vec<CpuFreqPolicy>, PowerProfileError> {
        let root = Path::new("/sys/devices/system/cpu/cpufreq");
        let entries = fs::read_dir(root)
            .map_err(|error| PowerProfileError::SysfsUnavailable(error.to_string()))?;
        let mut policies = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("policy"))
            .map(|entry| CpuFreqPolicy::read(entry.path()))
            .collect::<Result<Vec<_>, _>>()?;
        policies.sort_by(|left, right| left.path.cmp(&right.path));

        if policies.is_empty() {
            return Err(PowerProfileError::SysfsUnavailable(
                "no CPUFreq policies were found".into(),
            ));
        }

        Ok(policies)
    }

    fn sysfs_available_profiles() -> Result<Vec<PowerProfile>, PowerProfileError> {
        let mut available = Self::sysfs_policies()
            .map(|policies| {
                [
                    PowerProfile::Saver,
                    PowerProfile::Balanced,
                    PowerProfile::Performance,
                ]
                .into_iter()
                .filter(|profile| policies.iter().all(|policy| policy.supports(*profile)))
                .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if let Ok(platform) = PlatformProfile::read() {
            for profile in [
                PowerProfile::Saver,
                PowerProfile::Balanced,
                PowerProfile::Performance,
            ] {
                if platform.supports(profile) && !available.contains(&profile) {
                    available.push(profile);
                }
            }
        }

        if available.is_empty() {
            return Err(PowerProfileError::SysfsUnavailable(
                "neither CPUFreq nor the firmware platform_profile interface exposes supported profile controls".into(),
            ));
        }

        Ok(available)
    }

    fn sysfs_active_profile() -> Result<PowerProfile, PowerProfileError> {
        if let Ok(platform) = PlatformProfile::read() {
            if let Ok(active) = platform.active_profile() {
                return Ok(active);
            }
        }

        if let Ok(policies) = Self::sysfs_policies() {
            if let Some(profiles) = policies
                .iter()
                .map(CpuFreqPolicy::active_profile)
                .collect::<Option<Vec<_>>>()
            {
                if let Some(active) = profiles.first().copied() {
                    if profiles.iter().all(|profile| *profile == active) {
                        return Ok(active);
                    }
                }
            }
        }

        PlatformProfile::read()?.active_profile()
    }

    fn sysfs_set_profile(
        requested: PowerProfile,
        previous: Option<PowerProfile>,
    ) -> Result<PowerProfileChange, PowerProfileError> {
        if let Ok(platform) = PlatformProfile::read() {
            if platform.supports(requested) {
                return platform.set_profile(requested, previous);
            }
        }

        if let Ok(policies) = Self::sysfs_policies() {
            if policies.iter().all(|policy| policy.supports(requested)) {
                for policy in &policies {
                    policy.set_profile(requested)?;
                }

                let active = Self::sysfs_active_profile()?;
                if active != requested {
                    return Err(PowerProfileError::VerificationFailed {
                        requested: requested.as_str().to_owned(),
                        actual: active.as_str().to_owned(),
                    });
                }

                return Ok(PowerProfileChange { previous, active });
            }
        }

        PlatformProfile::read()?.set_profile(requested, previous)
    }
}

struct CpuFreqPolicy {
    path: PathBuf,
    governors: Vec<String>,
    epp_preferences: Vec<String>,
}

impl CpuFreqPolicy {
    fn read(path: PathBuf) -> Result<Self, PowerProfileError> {
        let governors = read_words(&path.join("scaling_available_governors"));
        let epp_preferences = read_words(&path.join("energy_performance_available_preferences"));

        if governors.is_empty() && epp_preferences.is_empty() {
            return Err(PowerProfileError::SysfsUnavailable(format!(
                "{} exposes no supported CPU profile controls",
                path.display()
            )));
        }

        Ok(Self {
            path,
            governors,
            epp_preferences,
        })
    }

    fn supports(&self, profile: PowerProfile) -> bool {
        self.epp_target(profile).is_some() || self.governor_target(profile).is_some()
    }

    fn epp_target(&self, profile: PowerProfile) -> Option<&str> {
        let candidates: &[&str] = match profile {
            PowerProfile::Saver => &["power", "power_save"],
            PowerProfile::Balanced => &["balance_performance", "balance_power"],
            PowerProfile::Performance => &["performance"],
        };
        candidates.iter().copied().find(|candidate| {
            self.epp_preferences
                .iter()
                .any(|available| available == candidate)
        })
    }

    fn governor_target(&self, profile: PowerProfile) -> Option<&str> {
        let candidates: &[&str] = match profile {
            PowerProfile::Saver => &["powersave"],
            PowerProfile::Balanced => &["schedutil", "ondemand", "conservative"],
            PowerProfile::Performance => &["performance"],
        };
        candidates.iter().copied().find(|candidate| {
            self.governors
                .iter()
                .any(|available| available == candidate)
        })
    }

    fn active_profile(&self) -> Option<PowerProfile> {
        let epp = fs::read_to_string(self.path.join("energy_performance_preference"))
            .ok()
            .map(|value| value.trim().to_owned());

        match epp.as_deref() {
            Some("power" | "power_save") => return Some(PowerProfile::Saver),
            Some("balance_performance" | "balance_power") => {
                return Some(PowerProfile::Balanced);
            }
            Some("performance") => return Some(PowerProfile::Performance),
            _ => {}
        }

        let governor = fs::read_to_string(self.path.join("scaling_governor")).ok()?;
        match governor.trim() {
            "powersave" => Some(PowerProfile::Saver),
            "performance" => Some(PowerProfile::Performance),
            "schedutil" | "ondemand" | "conservative" => Some(PowerProfile::Balanced),
            _ => None,
        }
    }

    fn set_profile(&self, profile: PowerProfile) -> Result<(), PowerProfileError> {
        let (filename, value) = if let Some(value) = self.epp_target(profile) {
            ("energy_performance_preference", value)
        } else if let Some(value) = self.governor_target(profile) {
            ("scaling_governor", value)
        } else {
            return Err(PowerProfileError::ProfileUnavailable(
                profile.as_str().to_owned(),
            ));
        };

        write_sysfs(&self.path.join(filename), value).map_err(|error| {
            PowerProfileError::SysfsUnavailable(format!(
                "cannot set {profile:?} in {}: {error}",
                self.path.display()
            ))
        })
    }
}

fn read_words(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

struct PlatformProfile {
    path: PathBuf,
    choices: Vec<String>,
}

impl PlatformProfile {
    fn read() -> Result<Self, PowerProfileError> {
        let path = PathBuf::from("/sys/firmware/acpi/platform_profile");
        let choices_path = PathBuf::from("/sys/firmware/acpi/platform_profile_choices");
        let choices = read_words(&choices_path);
        if choices.is_empty() {
            return Err(PowerProfileError::SysfsUnavailable(format!(
                "{} is missing or has no available choices",
                choices_path.display()
            )));
        }

        Ok(Self { path, choices })
    }

    fn target(&self, profile: PowerProfile) -> Option<&str> {
        let candidates: &[&str] = match profile {
            PowerProfile::Saver => &["low-power", "quiet", "cool"],
            PowerProfile::Balanced => &["balanced"],
            PowerProfile::Performance => &["performance", "balanced-performance"],
        };
        candidates
            .iter()
            .copied()
            .find(|candidate| self.choices.iter().any(|choice| choice == candidate))
    }

    fn supports(&self, profile: PowerProfile) -> bool {
        self.target(profile).is_some()
    }

    fn active_profile(&self) -> Result<PowerProfile, PowerProfileError> {
        let current = fs::read_to_string(&self.path).map_err(|error| {
            PowerProfileError::SysfsUnavailable(format!(
                "cannot read {}: {error}",
                self.path.display()
            ))
        })?;

        match current.trim() {
            "low-power" | "quiet" | "cool" => Ok(PowerProfile::Saver),
            "balanced" => Ok(PowerProfile::Balanced),
            "performance" | "balanced-performance" => Ok(PowerProfile::Performance),
            other => Err(PowerProfileError::SysfsUnavailable(format!(
                "unknown active platform profile: {other}"
            ))),
        }
    }

    fn set_profile(
        &self,
        profile: PowerProfile,
        previous: Option<PowerProfile>,
    ) -> Result<PowerProfileChange, PowerProfileError> {
        let target = self
            .target(profile)
            .ok_or_else(|| PowerProfileError::ProfileUnavailable(profile.as_str().into()))?;

        write_sysfs(&self.path, target).map_err(|error| {
            PowerProfileError::SysfsUnavailable(format!("cannot set {profile:?}: {error}"))
        })?;

        let active = self.active_profile()?;
        if active != profile {
            return Err(PowerProfileError::VerificationFailed {
                requested: profile.as_str().into(),
                actual: active.as_str().into(),
            });
        }

        Ok(PowerProfileChange { previous, active })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(governors: &[&str], epp_preferences: &[&str]) -> CpuFreqPolicy {
        CpuFreqPolicy {
            path: PathBuf::new(),
            governors: governors.iter().map(|value| (*value).into()).collect(),
            epp_preferences: epp_preferences
                .iter()
                .map(|value| (*value).into())
                .collect(),
        }
    }

    #[test]
    fn cpu_frequency_policies_support_profiles_with_epp() {
        let policy = policy(
            &["performance", "powersave"],
            &["performance", "balance_performance", "power"],
        );

        assert!(policy.supports(PowerProfile::Saver));
        assert!(policy.supports(PowerProfile::Balanced));
        assert!(policy.supports(PowerProfile::Performance));
        assert_eq!(policy.epp_target(PowerProfile::Saver), Some("power"));
        assert_eq!(
            policy.epp_target(PowerProfile::Balanced),
            Some("balance_performance")
        );
    }

    #[test]
    fn cpu_frequency_policies_fall_back_to_governors() {
        let policy = policy(&["performance", "powersave"], &[]);

        assert!(policy.supports(PowerProfile::Saver));
        assert!(!policy.supports(PowerProfile::Balanced));
        assert!(policy.supports(PowerProfile::Performance));
        assert_eq!(
            policy.governor_target(PowerProfile::Saver),
            Some("powersave")
        );
    }

    #[test]
    fn platform_profile_choices_map_to_available_profiles() {
        let platform = PlatformProfile {
            path: PathBuf::new(),
            choices: vec!["low-power".into(), "balanced".into(), "performance".into()],
        };

        assert!(platform.supports(PowerProfile::Saver));
        assert!(platform.supports(PowerProfile::Balanced));
        assert!(platform.supports(PowerProfile::Performance));
        assert_eq!(platform.target(PowerProfile::Saver), Some("low-power"));
        assert_eq!(platform.target(PowerProfile::Balanced), Some("balanced"));
    }
}
