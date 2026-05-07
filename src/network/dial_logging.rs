//! Dial-attempt logging per spec 005 US1 T020 / FR-004.
//!
//! Every dial attempt emitted from the swarm event loop MUST be visible at
//! `info` level or higher, never swallowed silently at `debug`/`trace`. This
//! module provides the canonical `DialAttempt` record and an emit helper so
//! every call site uses the same format.

use crate::types::{DialOutcome, TransportKind};
use chrono::{DateTime, Utc};
use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};

/// A single observable dial attempt record (data-model A.3).
///
/// Emitted via `emit_dial_event` at `info` level. Tests can capture these
/// events via a `tracing-subscriber` layer to verify coverage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialAttempt {
    pub timestamp: DateTime<Utc>,
    pub target_multiaddr: String,
    pub transport: TransportKind,
    pub outcome: DialOutcome,
    /// Present iff outcome != Success. Carries the root-cause string from the
    /// underlying transport (e.g. "Connection refused", "TLS handshake failed: ...").
    pub root_cause: Option<String>,
}

impl DialAttempt {
    /// Construct a success record. `root_cause` is always None.
    pub fn success(target: &Multiaddr, transport: TransportKind) -> Self {
        Self {
            timestamp: Utc::now(),
            target_multiaddr: target.to_string(),
            transport,
            outcome: DialOutcome::Success,
            root_cause: None,
        }
    }

    /// Construct a failure record. `root_cause` is required.
    pub fn failure(
        target: &Multiaddr,
        transport: TransportKind,
        outcome: DialOutcome,
        root_cause: impl Into<String>,
    ) -> Self {
        debug_assert!(
            !matches!(outcome, DialOutcome::Success),
            "use DialAttempt::success for successful dials"
        );
        Self {
            timestamp: Utc::now(),
            target_multiaddr: target.to_string(),
            transport,
            outcome,
            root_cause: Some(root_cause.into()),
        }
    }
}

/// Emit a dial-attempt record to the tracing subscriber at `info` level.
/// Failures are emitted with the full root_cause attached as a structured
/// field — never swallowed silently (FR-004).
pub fn emit_dial_event(ev: &DialAttempt) {
    match &ev.outcome {
        DialOutcome::Success => {
            tracing::info!(
                target = %ev.target_multiaddr,
                transport = ?ev.transport,
                "dial succeeded"
            );
        }
        DialOutcome::Timeout => {
            tracing::info!(
                target = %ev.target_multiaddr,
                transport = ?ev.transport,
                root_cause = ev.root_cause.as_deref().unwrap_or(""),
                "dial timed out"
            );
        }
        DialOutcome::TransportError(msg) => {
            tracing::info!(
                target = %ev.target_multiaddr,
                transport = ?ev.transport,
                detail = %msg,
                root_cause = ev.root_cause.as_deref().unwrap_or(""),
                "dial failed: transport error"
            );
        }
        DialOutcome::Denied(msg) => {
            tracing::info!(
                target = %ev.target_multiaddr,
                transport = ?ev.transport,
                detail = %msg,
                root_cause = ev.root_cause.as_deref().unwrap_or(""),
                "dial denied by remote"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn success_record_has_no_root_cause() {
        let addr = Multiaddr::from_str("/ip4/127.0.0.1/tcp/4001").unwrap();
        let ev = DialAttempt::success(&addr, TransportKind::Tcp);
        assert!(ev.root_cause.is_none());
        assert_eq!(ev.outcome, DialOutcome::Success);
    }

    #[test]
    fn failure_record_preserves_root_cause() {
        let addr = Multiaddr::from_str("/ip4/10.0.0.1/tcp/4001").unwrap();
        let ev = DialAttempt::failure(
            &addr,
            TransportKind::Tcp,
            DialOutcome::TransportError("ECONNREFUSED".into()),
            "connection refused by remote host",
        );
        assert_eq!(ev.root_cause.as_deref(), Some("connection refused by remote host"));
        assert!(matches!(ev.outcome, DialOutcome::TransportError(_)));
    }

    #[test]
    fn emit_does_not_panic_on_any_variant() {
        let addr = Multiaddr::from_str("/ip4/127.0.0.1/tcp/4001").unwrap();
        // Exercise each variant
        emit_dial_event(&DialAttempt::success(&addr, TransportKind::Tcp));
        emit_dial_event(&DialAttempt::failure(
            &addr,
            TransportKind::Quic,
            DialOutcome::Timeout,
            "no response within 30s",
        ));
        emit_dial_event(&DialAttempt::failure(
            &addr,
            TransportKind::Wss,
            DialOutcome::TransportError("tls handshake: middlebox".into()),
            "TLS cert pin mismatch",
        ));
        emit_dial_event(&DialAttempt::failure(
            &addr,
            TransportKind::Relay,
            DialOutcome::Denied("reservation quota exhausted".into()),
            "remote relay denied reservation",
        ));
    }
}
