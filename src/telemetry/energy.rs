//! Energy metering and carbon footprint estimation (FR-105).
//!
//! Reads Intel RAPL energy counters on Linux, estimates power draw,
//! and computes carbon footprint from regional intensity data.

use std::time::Instant;

/// A single energy reading from hardware counters.
#[derive(Debug, Clone)]
pub struct EnergyReading {
    /// CPU energy consumption in joules (from RAPL or equivalent).
    pub cpu_joules: f64,
    /// GPU instantaneous power draw in watts, if available.
    pub gpu_watts: Option<f64>,
    /// Timestamp of the reading.
    pub timestamp: Instant,
}

/// Read the CPU package energy counter from Intel RAPL.
///
/// Reads `/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj` and converts
/// microjoules to joules. Returns `Err` on non-Linux platforms or if RAPL is
/// unavailable (e.g., in CI, VMs, or non-Intel hardware).
pub fn read_rapl_energy() -> Result<f64, String> {
    #[cfg(target_os = "linux")]
    {
        let path = "/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj";
        let contents =
            std::fs::read_to_string(path).map_err(|e| format!("RAPL not available: {e}"))?;
        let microjoules: f64 =
            contents.trim().parse().map_err(|e| format!("Failed to parse RAPL value: {e}"))?;
        Ok(microjoules / 1_000_000.0)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err("RAPL is only available on Linux".to_string())
    }
}

/// Estimate power consumption in watts from energy and duration.
///
/// # Arguments
/// * `joules` — Total energy consumed in joules.
/// * `duration_secs` — Duration of the measurement window in seconds.
pub fn estimate_power_watts(joules: f64, duration_secs: f64) -> f64 {
    joules / duration_secs
}

/// Read GPU power draw via NVML.
///
/// Currently returns `Err` because NVML bindings are not yet integrated.
/// Future versions will query `nvmlDeviceGetPowerUsage` for NVIDIA GPUs.
pub fn read_gpu_power_watts() -> Result<f64, String> {
    Err("NVML not available".to_string())
}

/// Carbon footprint for a compute workload.
#[derive(Debug, Clone)]
pub struct CarbonFootprint {
    /// Energy consumed in kilowatt-hours.
    pub kwh: f64,
    /// Estimated CO2 emissions in grams.
    pub co2_grams: f64,
    /// Regional carbon intensity used for the estimate (gCO2/kWh).
    pub carbon_intensity_g_per_kwh: f64,
}

/// Compute the carbon footprint for a given energy consumption.
///
/// # Arguments
/// * `joules` — Total energy consumed in joules.
/// * `carbon_intensity` — Regional carbon intensity in grams CO2 per kWh.
pub fn compute_carbon(joules: f64, carbon_intensity: f64) -> CarbonFootprint {
    let kwh = joules / 3_600_000.0;
    let co2_grams = kwh * carbon_intensity;
    CarbonFootprint { kwh, co2_grams, carbon_intensity_g_per_kwh: carbon_intensity }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_power_watts() {
        let watts = estimate_power_watts(100.0, 10.0);
        assert!((watts - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_carbon_known_values() {
        // 3,600,000 J = 1 kWh; at 400 gCO2/kWh => 400 g CO2
        let footprint = compute_carbon(3_600_000.0, 400.0);
        assert!((footprint.kwh - 1.0).abs() < 1e-9);
        assert!((footprint.co2_grams - 400.0).abs() < 1e-9);
        assert!((footprint.carbon_intensity_g_per_kwh - 400.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compute_carbon_zero_joules() {
        let footprint = compute_carbon(0.0, 500.0);
        assert!((footprint.kwh).abs() < f64::EPSILON);
        assert!((footprint.co2_grams).abs() < f64::EPSILON);
    }

    #[test]
    fn test_gpu_power_not_available() {
        let result = read_gpu_power_watts();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "NVML not available");
    }
}
