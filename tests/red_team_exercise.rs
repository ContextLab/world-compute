//! T105 — GO/NO-GO GATE: Formal red team exercise (SC-S008).
//!
//! This automated adversarial test suite covers the 5 attack scenarios
//! required by the safety hardening spec before multi-institution deployment:
//!
//! 1. Malicious workload submission
//! 2. Compromised account
//! 3. Policy bypass attempt
//! 4. Sandbox escape attempt
//! 5. Supply-chain injection
//!
//! All scenarios must PASS (attacks rejected) for the GO/NO-GO gate.

mod red_team {
    mod scenario_1_malicious_workload;
    mod scenario_2_compromised_account;
    mod scenario_3_policy_bypass;
    mod scenario_4_sandbox_escape;
    mod scenario_5_supply_chain;
}
