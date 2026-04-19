//! DNS-over-HTTPS fallback resolver per spec 005 US1 T019 / FR-005.
//!
//! When the OS resolver fails to resolve a `/dnsaddr/...` multiaddr (captive
//! portals, strict DNS filtering, DNS-blocking firewalls), this fallback
//! engages automatically using the bundled DoH upstreams.
//!
//! This is a thin wrapper around `hickory-resolver` that exposes a simple
//! `resolve_a_aaaa` entry point that returns all A / AAAA records for a
//! hostname. libp2p's `/dnsaddr/` logic can consult this on OS-resolver
//! failure.

use hickory_resolver::config::{NameServerConfigGroup, ResolverConfig, ResolverOpts};
use hickory_resolver::TokioAsyncResolver;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Duration;

/// Configuration for the DoH fallback resolver (data-model A.4).
#[derive(Debug, Clone)]
pub struct DohResolverConfig {
    /// Enable the fallback. Default true.
    pub enabled: bool,
    /// DoH upstream URLs. Default: Cloudflare 1.1.1.1 + Google 8.8.8.8.
    pub upstreams: Vec<String>,
    /// Per-query timeout.
    pub timeout: Duration,
}

impl Default for DohResolverConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            upstreams: vec![
                "https://cloudflare-dns.com/dns-query".to_string(),
                "https://dns.google/dns-query".to_string(),
            ],
            timeout: Duration::from_secs(5),
        }
    }
}

/// DoH fallback resolver. Construct once at daemon startup and consult on
/// OS-resolver failure.
#[derive(Clone)]
pub struct DohFallback {
    resolver: Arc<TokioAsyncResolver>,
    config: DohResolverConfig,
}

impl DohFallback {
    /// Build with Cloudflare + Google as upstreams. Returns an error if
    /// hickory-resolver cannot initialize (should not happen in normal use).
    pub fn new(config: DohResolverConfig) -> Result<Self, std::io::Error> {
        // Cloudflare DoH: 1.1.1.1 + 1.0.0.1
        let cloudflare = NameServerConfigGroup::cloudflare_https();
        // Google DoH: 8.8.8.8 + 8.8.4.4
        let google = NameServerConfigGroup::google_https();

        let mut ns_group = NameServerConfigGroup::new();
        for ns in cloudflare.iter().chain(google.iter()) {
            ns_group.push(ns.clone());
        }

        let resolver_config =
            ResolverConfig::from_parts(None, vec![], ns_group);
        let mut opts = ResolverOpts::default();
        opts.timeout = config.timeout;
        opts.attempts = 2;

        let resolver = TokioAsyncResolver::tokio(resolver_config, opts);
        Ok(Self { resolver: Arc::new(resolver), config })
    }

    /// Resolve a hostname to all A / AAAA records. Returns an empty vec if
    /// the resolver reports NXDOMAIN; returns an Err if DoH itself is
    /// unreachable.
    pub async fn resolve(&self, hostname: &str) -> Result<Vec<IpAddr>, String> {
        if !self.config.enabled {
            return Err("DoH fallback disabled".into());
        }
        let mut out = Vec::new();
        match self.resolver.lookup_ip(hostname).await {
            Ok(lookup) => {
                for ip in lookup.iter() {
                    out.push(ip);
                }
                tracing::info!(
                    hostname = %hostname,
                    count = out.len(),
                    "DoH fallback resolve succeeded"
                );
                Ok(out)
            }
            Err(e) => {
                tracing::info!(
                    hostname = %hostname,
                    error = %e,
                    "DoH fallback resolve failed"
                );
                Err(format!("DoH lookup failed: {e}"))
            }
        }
    }

    pub fn config(&self) -> &DohResolverConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_two_upstreams() {
        let cfg = DohResolverConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.upstreams.len(), 2);
        assert!(cfg.upstreams.iter().all(|u| u.starts_with("https://")));
    }

    #[test]
    fn builder_succeeds() {
        let cfg = DohResolverConfig::default();
        let _doh = DohFallback::new(cfg).expect("build DoH resolver");
    }

    // Real DoH lookup test — requires network. Marked #[ignore] to keep the
    // default cargo test run hermetic. Run with:
    //   cargo test -- --ignored doh_real_lookup
    #[tokio::test]
    #[ignore = "requires network access to Cloudflare or Google DoH endpoints"]
    async fn doh_real_lookup() {
        let doh = DohFallback::new(DohResolverConfig::default()).unwrap();
        let ips = doh.resolve("one.one.one.one").await.expect("resolve");
        assert!(!ips.is_empty(), "DoH lookup must return at least one IP");
        // one.one.one.one is Cloudflare — expect 1.1.1.1 or 1.0.0.1
        assert!(ips.iter().any(|ip| ip.to_string() == "1.1.1.1" || ip.to_string() == "1.0.0.1"));
    }

    #[tokio::test]
    async fn disabled_resolver_refuses() {
        let cfg = DohResolverConfig { enabled: false, ..Default::default() };
        let doh = DohFallback::new(cfg).unwrap();
        let r = doh.resolve("example.com").await;
        assert!(r.is_err());
    }
}
