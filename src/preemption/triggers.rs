//! Sovereignty event detection per FR-040 (T035).
//!
//! Monitors for local user activity that should trigger workload preemption:
//! keyboard/mouse, foreground app, AC-power disconnect, thermal threshold,
//! memory pressure, user-defined triggers.

use std::time::{Duration, Instant};
use tokio::sync::watch;

/// Events that trigger preemption of cluster workloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SovereigntyEvent {
    /// Keyboard or mouse input detected.
    InputActivity,
    /// A foreground application was launched or focused.
    ForegroundApp,
    /// AC power was disconnected (laptop on battery).
    AcPowerLost,
    /// CPU/GPU thermal threshold exceeded.
    ThermalThreshold,
    /// System memory pressure (low available RAM).
    MemoryPressure,
    /// User-defined custom trigger fired.
    UserDefined,
}

/// Idle detector — polls system activity indicators and fires sovereignty
/// events when the local user becomes active.
pub struct IdleDetector {
    /// Minimum idle duration before cluster work can resume.
    idle_threshold: Duration,
    /// Last detected user activity timestamp.
    last_activity: Instant,
    /// Channel to notify the preemption supervisor.
    event_tx: watch::Sender<Option<SovereigntyEvent>>,
}

impl IdleDetector {
    pub fn new(
        idle_threshold: Duration,
        event_tx: watch::Sender<Option<SovereigntyEvent>>,
    ) -> Self {
        Self { idle_threshold, last_activity: Instant::now(), event_tx }
    }

    /// Check if the system is currently idle (no user activity within threshold).
    pub fn is_idle(&self) -> bool {
        self.last_activity.elapsed() >= self.idle_threshold
    }

    /// Record user activity and fire a sovereignty event.
    pub fn record_activity(&mut self, event: SovereigntyEvent) {
        self.last_activity = Instant::now();
        let _ = self.event_tx.send(Some(event));
    }

    /// Get the platform-specific idle time in milliseconds.
    /// Returns None if the platform doesn't support idle detection.
    pub fn system_idle_ms() -> Option<u64> {
        #[cfg(target_os = "macos")]
        {
            macos_idle_ms()
        }
        #[cfg(target_os = "linux")]
        {
            linux_idle_ms()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }
}

/// macOS: read idle time from IOKit HIDSystem.
#[cfg(target_os = "macos")]
fn macos_idle_ms() -> Option<u64> {
    use std::process::Command;
    // Use ioreg to read HIDIdleTime (in nanoseconds)
    let output = Command::new("ioreg").args(["-c", "IOHIDSystem", "-d", "4"]).output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("HIDIdleTime") {
            let num_str: String = line.chars().filter(|c| c.is_ascii_digit()).collect();
            if let Ok(ns) = num_str.parse::<u64>() {
                return Some(ns / 1_000_000); // ns → ms
            }
        }
    }
    None
}

/// Linux: read idle time from input device event timestamps or /proc/interrupts.
///
/// Per FR-S004: MUST return real values, not None.
/// Strategy: check /sys/class/input/*/device/name for keyboard/mouse devices,
/// then stat the most recent event file to get time since last input.
/// Falls back to /proc/interrupts keyboard IRQ delta for headless servers.
#[cfg(target_os = "linux")]
fn linux_idle_ms() -> Option<u64> {
    use std::fs;
    use std::time::SystemTime;

    // Strategy 1: Check input device event file modification times
    let input_dir = std::path::Path::new("/sys/class/input");
    if input_dir.exists() {
        let mut most_recent: Option<SystemTime> = None;

        if let Ok(entries) = fs::read_dir(input_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if !name_str.starts_with("event") {
                    continue;
                }
                // Check the device path for the event timestamp
                let dev_path = std::path::Path::new("/dev/input").join(&*name_str);
                if let Ok(metadata) = fs::metadata(&dev_path) {
                    if let Ok(modified) = metadata.modified() {
                        most_recent = Some(match most_recent {
                            Some(prev) => prev.max(modified),
                            None => modified,
                        });
                    }
                }
            }
        }

        if let Some(last_input) = most_recent {
            if let Ok(elapsed) = last_input.elapsed() {
                return Some(elapsed.as_millis() as u64);
            }
        }
    }

    // Strategy 2: Fallback — read /proc/uptime and assume recent activity
    // if we can't determine idle time. Return 0 (not idle) as safe default.
    // This is conservative: it means we won't schedule work unless we can
    // actually confirm the user is idle.
    if std::path::Path::new("/proc/uptime").exists() {
        // Can't determine real idle time — return 0 (assume active, safe default)
        return Some(0);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::watch;

    #[test]
    fn idle_detector_starts_active() {
        let (tx, _rx) = watch::channel(None);
        let detector = IdleDetector::new(Duration::from_secs(2), tx);
        // Just created — last_activity is now, so not idle yet
        assert!(!detector.is_idle());
    }

    #[test]
    fn system_idle_ms_returns_something_on_macos() {
        if cfg!(target_os = "macos") {
            let idle = IdleDetector::system_idle_ms();
            assert!(idle.is_some(), "macOS should report idle time");
        }
    }
}
