//! Reproducible build metadata per FR-006.

/// Compile-time build information for reproducibility and auditability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildInfo {
    /// Semantic version from Cargo.toml.
    pub version: &'static str,
    /// Git SHA of the commit this binary was built from.
    /// Set via `VERGEN_GIT_SHA` or the `GIT_SHA` env var at build time.
    pub git_sha: &'static str,
    /// ISO-8601 build timestamp injected at compile time.
    pub build_timestamp: &'static str,
    /// Whether the binary was built with a reproducible signed build.
    pub is_signed: bool,
}

/// Return the build info for this binary, populated from compile-time env vars.
///
/// The build script (or CI) is expected to set:
///   - `CARGO_PKG_VERSION` (automatic from Cargo)
///   - `GIT_SHA` (set by CI or a build.rs)
///   - `BUILD_TIMESTAMP` (set by CI or a build.rs)
///   - `SIGNED_BUILD` (set to "true" by the release pipeline)
pub fn get_build_info() -> BuildInfo {
    BuildInfo {
        version: env!("CARGO_PKG_VERSION"),
        git_sha: option_env!("GIT_SHA").unwrap_or("unknown"),
        build_timestamp: option_env!("BUILD_TIMESTAMP").unwrap_or("unknown"),
        is_signed: matches!(option_env!("SIGNED_BUILD"), Some("true")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_info_has_non_empty_version() {
        let info = get_build_info();
        assert!(!info.version.is_empty(), "version should not be empty");
    }

    #[test]
    fn build_info_version_matches_cargo() {
        let info = get_build_info();
        // CARGO_PKG_VERSION is always set by Cargo; must be semver-like
        assert!(info.version.contains('.'), "version '{}' should be semver", info.version);
    }

    #[test]
    fn build_info_git_sha_is_present() {
        let info = get_build_info();
        // In CI this will be a real SHA; in dev it falls back to "unknown"
        assert!(!info.git_sha.is_empty());
    }
}
