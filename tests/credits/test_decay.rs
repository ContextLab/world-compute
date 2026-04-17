//! Integration tests for credit decay (T123-T126).

use worldcompute::credits::decay::{apply_decay, compute_decay_event, CreditDecayConfig};
use worldcompute::types::NcuAmount;

#[test]
fn half_life_45_days_halves_balance() {
    // T126: 45-day half-life: balance 1000 after 45 days -> ~500
    let balance = NcuAmount::from_ncu(1000.0);
    let config = CreditDecayConfig::default(); // 45-day half-life
    let result = apply_decay(balance, 45.0, NcuAmount::ZERO, &config);
    assert!(
        (result.as_ncu() - 500.0).abs() < 1.0,
        "Expected ~500 NCU after 45 days, got {}",
        result.as_ncu()
    );
}

#[test]
fn floor_protection_active_donor() {
    // T126: Active donor doesn't go below floor
    // earn_rate = 10 NCU/day, floor = 10 * 30 = 300 NCU
    // balance = 400, after 200 days of decay should hit floor
    let balance = NcuAmount::from_ncu(400.0);
    let earn_rate = NcuAmount::from_ncu(10.0);
    let config = CreditDecayConfig::default();
    let result = apply_decay(balance, 200.0, earn_rate, &config);
    let floor = 10.0 * 30.0;
    assert!(
        result.as_ncu() >= floor - 0.01,
        "Floor protection failed: balance {} < floor {}",
        result.as_ncu(),
        floor
    );
}

#[test]
fn anti_hoarding_accelerates_decay() {
    // T126: High balance gets accelerated decay
    // balance = 10000, trailing_redemption = 100 (balance >> 1.1 * redemption)
    let peer_id = libp2p::PeerId::random();
    let balance = NcuAmount::from_ncu(10000.0);
    let earn_rate = NcuAmount::ZERO;
    let redemption = NcuAmount::from_ncu(100.0);
    let config = CreditDecayConfig::default();

    // Normal decay event (no anti-hoarding)
    let normal_result = apply_decay(balance, 45.0, earn_rate, &config);

    // Anti-hoarding decay event
    let event = compute_decay_event(peer_id, balance, 45.0, earn_rate, redemption, &config);

    // Anti-hoarding should produce a LOWER balance (faster decay)
    assert!(
        event.balance_after.as_ncu() < normal_result.as_ncu(),
        "Anti-hoarding should accelerate decay: {} should be < {}",
        event.balance_after.as_ncu(),
        normal_result.as_ncu()
    );
}

#[test]
fn compute_decay_event_produces_valid_event() {
    let peer_id = libp2p::PeerId::random();
    let balance = NcuAmount::from_ncu(1000.0);
    let earn_rate = NcuAmount::from_ncu(1.0);
    let redemption = NcuAmount::from_ncu(500.0);
    let config = CreditDecayConfig::default();

    let event = compute_decay_event(peer_id, balance, 10.0, earn_rate, redemption, &config);

    assert_eq!(event.account_id, peer_id);
    assert_eq!(event.balance_before, balance);
    assert!(event.balance_after.as_ncu() < balance.as_ncu());
    assert!(event.balance_after.as_ncu() > 0.0);
    assert!(event.decay_rate > 0.0 && event.decay_rate < 1.0);
}

#[test]
fn no_anti_hoarding_when_balance_below_threshold() {
    // When balance <= 1.1 * trailing_redemption, no acceleration
    let peer_id = libp2p::PeerId::random();
    let balance = NcuAmount::from_ncu(100.0);
    let earn_rate = NcuAmount::ZERO;
    let redemption = NcuAmount::from_ncu(100.0); // balance == 1.0 * redemption
    let config = CreditDecayConfig::default();

    let event = compute_decay_event(peer_id, balance, 45.0, earn_rate, redemption, &config);
    let normal = apply_decay(balance, 45.0, earn_rate, &config);

    // Should be the same — no acceleration
    assert!(
        (event.balance_after.as_ncu() - normal.as_ncu()).abs() < 0.01,
        "Should not accelerate: event={} normal={}",
        event.balance_after.as_ncu(),
        normal.as_ncu()
    );
}
