use serde::Serialize;
use std::fs;

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub used_bytes: u64,
    pub cached_bytes: u64,
    pub buffers_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub usage_percent: f64,
    pub swap_usage_percent: f64,
}

pub fn read_memory() -> MemoryInfo {
    let Ok(text) = fs::read_to_string("/proc/meminfo") else {
        return MemoryInfo::default();
    };

    let mut values = std::collections::HashMap::<String, u64>::new();

    for line in text.lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };

        if let Some(kb) = rest
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<u64>().ok())
        {
            values.insert(key.to_string(), kb.saturating_mul(1024));
        }
    }

    let get = |key: &str| values.get(key).copied().unwrap_or(0);

    let total = get("MemTotal");
    let available = get("MemAvailable").min(total);
    let used = total.saturating_sub(available);
    let swap_total = get("SwapTotal");
    let swap_free = get("SwapFree").min(swap_total);
    let swap_used = swap_total.saturating_sub(swap_free);

    MemoryInfo {
        total_bytes: total,
        available_bytes: available,
        used_bytes: used,
        cached_bytes: get("Cached"),
        buffers_bytes: get("Buffers"),
        swap_total_bytes: swap_total,
        swap_used_bytes: swap_used,
        usage_percent: if total > 0 {
            used as f64 / total as f64 * 100.0
        } else {
            0.0
        },
        swap_usage_percent: if swap_total > 0 {
            swap_used as f64 / swap_total as f64 * 100.0
        } else {
            0.0
        },
    }
}
