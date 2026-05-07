//! Relay v2 reservation lifecycle per spec 005 US1 T021 / FR-002, FR-006, FR-007.
//!
//! A `RelayReservation` represents a libp2p Relay v2 reservation held by this
//! agent on a remote relay so NATed peers can reach it via a circuit address.
//!
//! Reservations expire and must be renewed before expiry. If a reservation is
//! lost (relay reboot, connection drop), the agent MUST reacquire from an
//! alternate relay within 60 s per FR-006. This module provides the state
//! machine + policy; the daemon event loop drives transitions based on real
//! libp2p swarm events.

use crate::types::ReservationStatus;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use libp2p::{Multiaddr, PeerId};

/// Maximum time allowed between detecting reservation loss and re-acquiring
/// from an alternate relay (FR-006).
pub const MAX_REACQUIRE_SECONDS: i64 = 60;

/// How far before `expires_at` we schedule a renewal. Must be comfortably
/// larger than a single round-trip so renewal arrives before expiry.
pub const RENEW_BEFORE_EXPIRY_SECONDS: i64 = 30;

/// A libp2p Relay v2 reservation held by this agent (data-model A.1).
#[derive(Debug, Clone)]
pub struct RelayReservation {
    /// The relay server's PeerId.
    pub relay_peer_id: PeerId,
    /// The reserved circuit address `/p2p/<relay>/p2p-circuit/p2p/<self>`.
    pub circuit_multiaddr: Multiaddr,
    /// Absolute expiry timestamp from the relay.
    pub expires_at: DateTime<Utc>,
    /// When we should kick off renewal (derived: expires_at - RENEW_BEFORE_EXPIRY).
    pub renew_at: DateTime<Utc>,
    /// Current state.
    pub status: ReservationStatus,
    /// Set when reservation is detected lost (Lost state only).
    pub lost_at: Option<DateTime<Utc>>,
}

impl RelayReservation {
    /// Construct a reservation in `Requesting` state.
    pub fn requesting(relay_peer_id: PeerId, circuit_multiaddr: Multiaddr) -> Self {
        let now = Utc::now();
        Self {
            relay_peer_id,
            circuit_multiaddr,
            expires_at: now, // unknown until accept
            renew_at: now,
            status: ReservationStatus::Requesting,
            lost_at: None,
        }
    }

    /// Transition to `Active` when the relay accepts the reservation.
    /// `ttl_seconds` is the relay-advertised lease length.
    pub fn mark_active(&mut self, ttl_seconds: i64) {
        let now = Utc::now();
        self.expires_at = now + ChronoDuration::seconds(ttl_seconds);
        self.renew_at = self.expires_at - ChronoDuration::seconds(RENEW_BEFORE_EXPIRY_SECONDS);
        self.status = ReservationStatus::Active;
        self.lost_at = None;
    }

    /// Transition to `Renewing` when we send a renewal request.
    pub fn mark_renewing(&mut self) {
        if self.status == ReservationStatus::Active {
            self.status = ReservationStatus::Renewing;
        }
    }

    /// Transition to `Lost` when we detect the reservation has dropped.
    /// Records `lost_at` so the reacquisition-deadline check can succeed.
    pub fn mark_lost(&mut self) {
        self.status = ReservationStatus::Lost;
        self.lost_at = Some(Utc::now());
    }

    /// Transition to `Failed` when the relay denies our request.
    pub fn mark_failed(&mut self) {
        self.status = ReservationStatus::Failed;
    }

    /// True iff the reservation is active and not yet at its `renew_at` threshold.
    pub fn is_healthy(&self, now: DateTime<Utc>) -> bool {
        self.status == ReservationStatus::Active && now < self.renew_at
    }

    /// True iff a renewal should be kicked off now.
    pub fn needs_renewal(&self, now: DateTime<Utc>) -> bool {
        self.status == ReservationStatus::Active && now >= self.renew_at && now < self.expires_at
    }

    /// Seconds elapsed since `lost_at`. Returns None if not Lost.
    pub fn time_since_lost(&self, now: DateTime<Utc>) -> Option<i64> {
        self.lost_at.map(|t| (now - t).num_seconds())
    }

    /// True iff we are within the 60-second reacquisition window after loss.
    pub fn within_reacquire_budget(&self, now: DateTime<Utc>) -> bool {
        self.time_since_lost(now).is_some_and(|secs| secs < MAX_REACQUIRE_SECONDS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn test_addr(suffix: &str) -> Multiaddr {
        Multiaddr::from_str(&format!(
            "/ip4/10.0.0.1/tcp/4001/p2p/{suffix}/p2p-circuit/p2p/{suffix}"
        ))
        .unwrap_or_else(|_| Multiaddr::from_str("/ip4/10.0.0.1/tcp/4001").unwrap())
    }

    #[test]
    fn new_reservation_is_requesting() {
        let peer = PeerId::random();
        let r = RelayReservation::requesting(peer, test_addr("abc"));
        assert_eq!(r.status, ReservationStatus::Requesting);
        assert!(r.lost_at.is_none());
    }

    #[test]
    fn active_transition_sets_deadlines() {
        let peer = PeerId::random();
        let mut r = RelayReservation::requesting(peer, test_addr("abc"));
        r.mark_active(300);
        assert_eq!(r.status, ReservationStatus::Active);
        // renew_at must be before expires_at
        assert!(r.renew_at < r.expires_at);
        // expires_at should be ~300s in the future
        let delta = (r.expires_at - Utc::now()).num_seconds();
        assert!(delta > 290 && delta <= 300);
    }

    #[test]
    fn healthy_until_renew_threshold() {
        let peer = PeerId::random();
        let mut r = RelayReservation::requesting(peer, test_addr("abc"));
        r.mark_active(300);
        assert!(r.is_healthy(Utc::now()));
        // Simulate time passing past renew_at
        let past_renew = r.renew_at + ChronoDuration::seconds(1);
        assert!(!r.is_healthy(past_renew));
        assert!(r.needs_renewal(past_renew));
    }

    #[test]
    fn lost_state_records_timestamp() {
        let peer = PeerId::random();
        let mut r = RelayReservation::requesting(peer, test_addr("abc"));
        r.mark_active(300);
        r.mark_lost();
        assert_eq!(r.status, ReservationStatus::Lost);
        assert!(r.lost_at.is_some());
        assert!(r.within_reacquire_budget(Utc::now()));
    }

    #[test]
    fn reacquire_budget_expires_after_60s() {
        let peer = PeerId::random();
        let mut r = RelayReservation::requesting(peer, test_addr("abc"));
        r.mark_active(300);
        r.mark_lost();
        let after_window = r.lost_at.unwrap() + ChronoDuration::seconds(MAX_REACQUIRE_SECONDS + 1);
        assert!(!r.within_reacquire_budget(after_window));
    }

    #[test]
    fn failed_state_is_terminal() {
        let peer = PeerId::random();
        let mut r = RelayReservation::requesting(peer, test_addr("abc"));
        r.mark_failed();
        assert_eq!(r.status, ReservationStatus::Failed);
    }
}
