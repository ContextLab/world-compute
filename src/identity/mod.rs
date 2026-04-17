//! Identity verification — Humanity Points verification flows.
//!
//! Per FR-S070–FR-S073: implements real verification backends for
//! proof-of-personhood, OAuth2 (email, phone, social), and Ed25519
//! key revocation. Verification occurs at enrollment time and is
//! re-verified at trust score recalculation intervals.

pub mod oauth2;
pub mod personhood;
pub mod phone;
