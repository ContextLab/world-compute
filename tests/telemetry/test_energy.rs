//! Integration tests for the energy metering module (T182).

use worldcompute::telemetry::energy;

#[test]
fn test_estimate_power_watts() {
    let watts = energy::estimate_power_watts(100.0, 10.0);
    assert!(
        (watts - 10.0).abs() < f64::EPSILON,
        "100 J / 10 s should equal 10 W, got {watts}"
    );
}

#[test]
fn test_estimate_power_watts_fractional() {
    let watts = energy::estimate_power_watts(50.0, 3.0);
    assert!(
        (watts - 50.0 / 3.0).abs() < 1e-12,
        "50 J / 3 s should equal ~16.667 W, got {watts}"
    );
}

#[test]
fn test_compute_carbon_known_values() {
    // 3,600,000 J = 1 kWh; at 400 gCO2/kWh => 400 g CO2
    let footprint = energy::compute_carbon(3_600_000.0, 400.0);
    assert!(
        (footprint.kwh - 1.0).abs() < 1e-9,
        "3.6 MJ should be 1 kWh, got {}",
        footprint.kwh
    );
    assert!(
        (footprint.co2_grams - 400.0).abs() < 1e-9,
        "1 kWh * 400 gCO2/kWh should be 400 g, got {}",
        footprint.co2_grams
    );
}

#[test]
fn test_compute_carbon_us_average_intensity() {
    // US average ~390 gCO2/kWh; 7200 J = 0.002 kWh => 0.78 g CO2
    let footprint = energy::compute_carbon(7200.0, 390.0);
    let expected_kwh = 7200.0 / 3_600_000.0;
    assert!(
        (footprint.kwh - expected_kwh).abs() < 1e-12,
        "kwh mismatch"
    );
    let expected_co2 = expected_kwh * 390.0;
    assert!(
        (footprint.co2_grams - expected_co2).abs() < 1e-9,
        "co2 mismatch"
    );
}

#[test]
fn test_read_rapl_energy_graceful_degradation() {
    // In CI and on non-Linux platforms, RAPL should return Err gracefully.
    let result = energy::read_rapl_energy();
    // We don't assert success — RAPL requires specific hardware and OS.
    // We only verify it doesn't panic and returns a well-formed Result.
    match result {
        Ok(joules) => {
            assert!(joules >= 0.0, "RAPL energy should be non-negative");
        }
        Err(msg) => {
            assert!(!msg.is_empty(), "Error message should be non-empty");
        }
    }
}

#[test]
fn test_gpu_power_returns_err() {
    let result = energy::read_gpu_power_watts();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "NVML not available");
}

// T182a: Calibration note
// -----------------------
// For hardware validation, compare read_rapl_energy() deltas against a
// wall-meter reading on tensor01 (Intel Xeon W-2295, Kill-A-Watt P4400).
// Expected agreement within 10% for sustained CPU-bound workloads.
// RAPL package domain covers CPU+uncore but not DRAM or peripherals,
// so wall-meter will read higher by ~20-40 W at load.
