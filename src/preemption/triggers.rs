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

/// Linux: read idle time from /proc or X11/Wayland idle APIs.
#[cfg(target_os = "linux")]
fn linux_idle_ms() -> Option<u64> {
    // TODO: Read from X11 XScreenSaverInfo or /sys/class/input/*/event timestamps.
    // For headless servers, consider checking /proc/stat for CPU idle transitions.
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
