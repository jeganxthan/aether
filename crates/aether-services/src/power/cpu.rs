use serde::Serialize;
use std::{
    fs,
    sync::{Mutex, OnceLock},
};

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CoreUsage {
    pub name: String,
    pub usage_percent: f64,
    pub frequency_mhz: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    pub usage_percent: f64,
    pub logical_cores: usize,
    pub load_average: [f64; 3],
    pub cores: Vec<CoreUsage>,
}

#[derive(Clone, Copy, Default)]
struct CpuTicks {
    total: u64,
    idle: u64,
}

static PREVIOUS: OnceLock<Mutex<Vec<CpuTicks>>> = OnceLock::new();

fn read_ticks() -> Vec<CpuTicks> {
    let Ok(text) = fs::read_to_string("/proc/stat") else {
        return Vec::new();
    };

    text.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let name = fields.next()?;

            if !name.starts_with("cpu") || name == "cpu" {
                return None;
            }

            let values: Vec<u64> = fields.filter_map(|field| field.parse().ok()).collect();

            if values.len() < 4 {
                return None;
            }

            let total = values.iter().sum();
            let idle = values[3] + values.get(4).copied().unwrap_or(0);

            Some(CpuTicks { total, idle })
        })
        .collect()
}

fn usage(previous: CpuTicks, current: CpuTicks) -> f64 {
    let total = current.total.saturating_sub(previous.total);
    let idle = current.idle.saturating_sub(previous.idle);

    if total == 0 {
        0.0
    } else {
        ((total.saturating_sub(idle)) as f64 / total as f64 * 100.0).clamp(0.0, 100.0)
    }
}

fn frequency(index: usize) -> Option<f64> {
    let path = format!("/sys/devices/system/cpu/cpu{index}/cpufreq/scaling_cur_freq");

    fs::read_to_string(path)
        .ok()?
        .trim()
        .parse::<f64>()
        .ok()
        .map(|khz| khz / 1000.0)
}

fn load_average() -> [f64; 3] {
    let Ok(text) = fs::read_to_string("/proc/loadavg") else {
        return [0.0; 3];
    };

    let values: Vec<f64> = text
        .split_whitespace()
        .take(3)
        .filter_map(|value| value.parse().ok())
        .collect();

    [
        *values.first().unwrap_or(&0.0),
        *values.get(1).unwrap_or(&0.0),
        *values.get(2).unwrap_or(&0.0),
    ]
}

pub fn read_cpu() -> CpuInfo {
    let current = read_ticks();

    if current.is_empty() {
        return CpuInfo::default();
    }

    let history = PREVIOUS.get_or_init(|| Mutex::new(Vec::new()));
    let mut previous = history.lock().unwrap_or_else(|error| error.into_inner());

    let cores = current
        .iter()
        .enumerate()
        .map(|(index, ticks)| {
            let usage_percent = previous
                .get(index)
                .copied()
                .map(|old| usage(old, *ticks))
                .unwrap_or(0.0);

            CoreUsage {
                name: format!("CPU {index}"),
                usage_percent,
                frequency_mhz: frequency(index),
            }
        })
        .collect::<Vec<_>>();

    let usage_percent = if previous.len() == current.len() {
        let old_total = previous.iter().fold(CpuTicks::default(), |mut acc, item| {
            acc.total += item.total;
            acc.idle += item.idle;
            acc
        });

        let new_total = current.iter().fold(CpuTicks::default(), |mut acc, item| {
            acc.total += item.total;
            acc.idle += item.idle;
            acc
        });

        usage(old_total, new_total)
    } else {
        0.0
    };

    *previous = current;

    CpuInfo {
        usage_percent,
        logical_cores: cores.len(),
        load_average: load_average(),
        cores,
    }
}
