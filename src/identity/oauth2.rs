//! OAuth2 verification flows for Humanity Points.
//!
//! Per FR-S073: implements real OAuth2 verification for email and
//! social account linking. Verified at enrollment, re-verified at
//! trust score recalculation intervals.
//!
//! Supports GitHub, Google, Twitter, and Email providers via the
//! `oauth2` crate (v4) authorization code flow. Provider credentials
//! are loaded from environment variables:
//!   OAUTH2_{PROVIDER}_CLIENT_ID
//!   OAUTH2_{PROVIDER}_CLIENT_SECRET

use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl,
};

/// OAuth2 provider types supported for HP verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OAuth2Provider {
    Email,
    GitHub,
    Google,
    Twitter,
}

impl OAuth2Provider {
    /// Environment variable prefix for this provider (uppercased).
    fn env_prefix(&self) -> &'static str {
        match self {
            OAuth2Provider::Email => "EMAIL",
            OAuth2Provider::GitHub => "GITHUB",
            OAuth2Provider::Google => "GOOGLE",
            OAuth2Provider::Twitter => "TWITTER",
        }
    }

    /// Well-known authorization URL for this provider.
    fn default_auth_url(&self) -> &'static str {
        match self {
            OAuth2Provider::Email => "https://accounts.google.com/o/oauth2/v2/auth",
            OAuth2Provider::GitHub => "https://github.com/login/oauth/authorize",
            OAuth2Provider::Google => "https://accounts.google.com/o/oauth2/v2/auth",
            OAuth2Provider::Twitter => "https://twitter.com/i/oauth2/authorize",
        }
    }

    /// Well-known token URL for this provider.
    fn default_token_url(&self) -> &'static str {
        match self {
            OAuth2Provider::Email => "https://oauth2.googleapis.com/token",
            OAuth2Provider::GitHub => "https://github.com/login/oauth/access_token",
            OAuth2Provider::Google => "https://oauth2.googleapis.com/token",
            OAuth2Provider::Twitter => "https://api.twitter.com/2/oauth2/token",
        }
    }

    /// Default scopes for this provider.
    fn default_scopes(&self) -> Vec<&'static str> {
        match self {
            OAuth2Provider::Email => vec!["email", "openid"],
            OAuth2Provider::GitHub => vec!["read:user", "user:email"],
            OAuth2Provider::Google => vec!["email", "openid", "profile"],
            OAuth2Provider::Twitter => vec!["users.read", "tweet.read"],
        }
    }
}

/// Configuration for an OAuth2 provider, loaded from environment variables.
#[derive(Debug, Clone)]
pub struct OAuth2ProviderConfig {
    /// Provider name.
    pub provider: OAuth2Provider,
    /// OAuth2 client ID.
    pub client_id: String,
    /// OAuth2 client secret.
    pub client_secret: String,
    /// Authorization endpoint URL.
    pub auth_url: String,
    /// Token endpoint URL.
    pub token_url: String,
    /// Redirect URI for the callback.
    pub redirect_uri: String,
    /// Scopes to request.
    pub scopes: Vec<String>,
}

impl OAuth2ProviderConfig {
    /// Load configuration from environment variables.
    ///
    /// Reads `OAUTH2_{PROVIDER}_CLIENT_ID` and `OAUTH2_{PROVIDER}_CLIENT_SECRET`.
    /// Auth/token URLs default to well-known provider endpoints but can be
    /// overridden via `OAUTH2_{PROVIDER}_AUTH_URL` and `OAUTH2_{PROVIDER}_TOKEN_URL`.
    ///
    /// Returns `None` if required credentials (client_id, client_secret) are missing.
    pub fn from_env(provider: OAuth2Provider, redirect_uri: &str) -> Option<Self> {
        let prefix = provider.env_prefix();
        let client_id = std::env::var(format!("OAUTH2_{prefix}_CLIENT_ID")).ok()?;
        let client_secret = std::env::var(format!("OAUTH2_{prefix}_CLIENT_SECRET")).ok()?;

        let auth_url = std::env::var(format!("OAUTH2_{prefix}_AUTH_URL"))
            .unwrap_or_else(|_| provider.default_auth_url().to_string());
        let token_url = std::env::var(format!("OAUTH2_{prefix}_TOKEN_URL"))
            .unwrap_or_else(|_| provider.default_token_url().to_string());

        let scopes = provider
            .default_scopes()
            .into_iter()
            .map(String::from)
            .collect();

        Some(Self {
            provider,
            client_id,
            client_secret,
            auth_url,
            token_url,
            redirect_uri: redirect_uri.to_string(),
            scopes,
        })
    }

    /// Build an `oauth2::BasicClient` from this configuration.
    pub fn build_client(&self) -> Result<BasicClient, String> {
        let client = BasicClient::new(
            ClientId::new(self.client_id.clone()),
            Some(ClientSecret::new(self.client_secret.clone())),
            AuthUrl::new(self.auth_url.clone())
                .map_err(|e| format!("Invalid auth URL: {e}"))?,
            Some(
                TokenUrl::new(self.token_url.clone())
                    .map_err(|e| format!("Invalid token URL: {e}"))?,
            ),
        )
        .set_redirect_uri(
            RedirectUrl::new(self.redirect_uri.clone())
                .map_err(|e| format!("Invalid redirect URI: {e}"))?,
        );
        Ok(client)
    }
}

/// Result of an OAuth2 verification flow.
#[derive(Debug, Clone)]
pub enum OAuth2Result {
    /// Successfully verified — provider confirmed the account.
    Verified {
        provider: OAuth2Provider,
        account_id: String,
    },
    /// Verification failed (e.g., invalid token, denied).
    Failed(String),
    /// Provider is unavailable (credentials missing or service unreachable).
    ProviderUnavailable(String),
}

/// Generate an authorization URL for the given provider.
///
/// Returns `(auth_url, csrf_token)` on success, or an error message.
pub fn generate_auth_url(
    provider: OAuth2Provider,
    redirect_uri: &str,
) -> Result<(String, String), String> {
    let config = OAuth2ProviderConfig::from_env(provider, redirect_uri).ok_or_else(|| {
        format!(
            "OAuth2 credentials not configured for {:?}. Set OAUTH2_{}_CLIENT_ID and OAUTH2_{}_CLIENT_SECRET environment variables.",
            provider,
            provider.env_prefix(),
            provider.env_prefix()
        )
    })?;

    let client = config.build_client()?;

    let mut auth_request = client.authorize_url(CsrfToken::new_random);
    for scope in &config.scopes {
        auth_request = auth_request.add_scope(Scope::new(scope.clone()));
    }
    let (url, csrf_token) = auth_request.url();

    Ok((url.to_string(), csrf_token.secret().clone()))
}

/// Exchange an authorization code for an access token and retrieve the user's
/// account ID from the provider.
///
/// This function performs the full OAuth2 authorization code exchange using
/// the `oauth2` crate, then queries the provider's user-info endpoint to
/// obtain the account identifier.
pub fn exchange_code(
    provider: OAuth2Provider,
    redirect_uri: &str,
    code: &str,
) -> OAuth2Result {
    let config = match OAuth2ProviderConfig::from_env(provider, redirect_uri) {
        Some(c) => c,
        None => {
            return OAuth2Result::ProviderUnavailable(format!(
                "OAuth2 credentials not configured for {:?}. Set OAUTH2_{}_CLIENT_ID and OAUTH2_{}_CLIENT_SECRET.",
                provider,
                provider.env_prefix(),
                provider.env_prefix()
            ));
        }
    };

    let client = match config.build_client() {
        Ok(c) => c,
        Err(e) => return OAuth2Result::Failed(format!("Failed to build OAuth2 client: {e}")),
    };

    // Exchange code for token using the oauth2 crate's built-in blocking HTTP client
    let http_client = oauth2::reqwest::http_client;
    let token_result = client
        .exchange_code(oauth2::AuthorizationCode::new(code.to_string()))
        .request(http_client);

    let token_response = match token_result {
        Ok(t) => t,
        Err(e) => return OAuth2Result::Failed(format!("Token exchange failed: {e}")),
    };

    let access_token = token_response.access_token().secret().clone();

    // Fetch user info from provider-specific endpoint
    match fetch_account_id(provider, &access_token) {
        Ok(account_id) => OAuth2Result::Verified {
            provider,
            account_id,
        },
        Err(e) => OAuth2Result::Failed(format!("Failed to fetch user info: {e}")),
    }
}

/// Fetch the account ID from the provider's user-info endpoint.
fn fetch_account_id(provider: OAuth2Provider, access_token: &str) -> Result<String, String> {
    let http_client = reqwest::blocking::Client::new();

    let (url, id_field) = match provider {
        OAuth2Provider::GitHub => ("https://api.github.com/user", "id"),
        OAuth2Provider::Google => (
            "https://www.googleapis.com/oauth2/v2/userinfo",
            "id",
        ),
        OAuth2Provider::Twitter => ("https://api.twitter.com/2/users/me", "id"),
        OAuth2Provider::Email => (
            "https://www.googleapis.com/oauth2/v2/userinfo",
            "email",
        ),
    };

    let response = http_client
        .get(url)
        .bearer_auth(access_token)
        .header("User-Agent", "world-compute/0.1")
        .header("Accept", "application/json")
        .send()
        .map_err(|e| format!("HTTP request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Provider returned HTTP {}",
            response.status()
        ));
    }

    let body: serde_json::Value = response
        .json()
        .map_err(|e| format!("Failed to parse response: {e}"))?;

    // Twitter nests user data under "data"
    let user_data = if provider == OAuth2Provider::Twitter {
        body.get("data").unwrap_or(&body)
    } else {
        &body
    };

    user_data
        .get(id_field)
        .and_then(|v| {
            if v.is_string() {
                v.as_str().map(String::from)
            } else {
                // GitHub returns numeric ID
                Some(v.to_string())
            }
        })
        .ok_or_else(|| format!("Field '{id_field}' not found in provider response"))
}

/// Initiate OAuth2 verification for the given provider.
///
/// When provider credentials are configured (via environment variables),
/// this generates an authorization URL. When credentials are missing,
/// it returns `ProviderUnavailable` with a descriptive message (T050).
///
/// In a full interactive flow the caller would:
/// 1. Call `verify_oauth2()` to get the auth URL
/// 2. Redirect the user to that URL
/// 3. Receive the callback with the authorization code
/// 4. Call `exchange_code()` to complete verification
pub fn verify_oauth2(provider: OAuth2Provider, redirect_uri: &str) -> OAuth2Result {
    match OAuth2ProviderConfig::from_env(provider, redirect_uri) {
        None => OAuth2Result::ProviderUnavailable(format!(
            "OAuth2 credentials not configured for {:?}. \
             Set OAUTH2_{}_CLIENT_ID and OAUTH2_{}_CLIENT_SECRET environment variables (see T088).",
            provider,
            provider.env_prefix(),
            provider.env_prefix()
        )),
        Some(config) => {
            // Credentials are available — try to build the client and generate auth URL
            match config.build_client() {
                Err(e) => OAuth2Result::ProviderUnavailable(format!(
                    "OAuth2 client configuration error for {provider:?}: {e}"
                )),
                Ok(client) => {
                    let mut auth_request = client.authorize_url(CsrfToken::new_random);
                    for scope in &config.scopes {
                        auth_request = auth_request.add_scope(Scope::new(scope.clone()));
                    }
                    let (url, _csrf_token) = auth_request.url();

                    // Return as "Failed" with the auth URL — the caller needs to
                    // redirect the user and then call exchange_code() with the code.
                    // In a non-interactive context, we cannot complete the flow.
                    OAuth2Result::Failed(format!(
                        "Authorization required. Visit: {url}"
                    ))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_env_prefix_matches() {
        assert_eq!(OAuth2Provider::GitHub.env_prefix(), "GITHUB");
        assert_eq!(OAuth2Provider::Google.env_prefix(), "GOOGLE");
        assert_eq!(OAuth2Provider::Twitter.env_prefix(), "TWITTER");
        assert_eq!(OAuth2Provider::Email.env_prefix(), "EMAIL");
    }

    #[test]
    fn config_from_env_returns_none_when_missing() {
        // With no env vars set, config should be None
        let config = OAuth2ProviderConfig::from_env(
            OAuth2Provider::GitHub,
            "https://localhost/callback",
        );
        // This will be None unless someone has OAUTH2_GITHUB_CLIENT_ID set
        if std::env::var("OAUTH2_GITHUB_CLIENT_ID").is_err() {
            assert!(config.is_none());
        }
    }

    #[test]
    fn verify_oauth2_returns_unavailable_without_credentials() {
        // Ensure the env vars are not set for this test
        if std::env::var("OAUTH2_GITHUB_CLIENT_ID").is_err() {
            match verify_oauth2(OAuth2Provider::GitHub, "https://localhost/callback") {
                OAuth2Result::ProviderUnavailable(msg) => {
                    assert!(msg.contains("OAUTH2_GITHUB_CLIENT_ID"));
                    assert!(msg.contains("T088"));
                }
                other => panic!("Expected ProviderUnavailable, got {other:?}"),
            }
        }
    }

    #[test]
    fn all_providers_unavailable_without_env() {
        for provider in [
            OAuth2Provider::Email,
            OAuth2Provider::GitHub,
            OAuth2Provider::Google,
            OAuth2Provider::Twitter,
        ] {
            if std::env::var(format!("OAUTH2_{}_CLIENT_ID", provider.env_prefix())).is_err() {
                assert!(
                    matches!(
                        verify_oauth2(provider, "https://localhost/callback"),
                        OAuth2Result::ProviderUnavailable(_)
                    ),
                    "Provider {:?} should be unavailable without env vars",
                    provider
                );
            }
        }
    }

    #[test]
    fn default_scopes_are_nonempty() {
        for provider in [
            OAuth2Provider::Email,
            OAuth2Provider::GitHub,
            OAuth2Provider::Google,
            OAuth2Provider::Twitter,
        ] {
            assert!(
                !provider.default_scopes().is_empty(),
                "Provider {:?} should have default scopes",
                provider
            );
        }
    }

    #[test]
    fn default_urls_are_valid() {
        for provider in [
            OAuth2Provider::Email,
            OAuth2Provider::GitHub,
            OAuth2Provider::Google,
            OAuth2Provider::Twitter,
        ] {
            assert!(provider.default_auth_url().starts_with("https://"));
            assert!(provider.default_token_url().starts_with("https://"));
        }
    }

    #[test]
    fn exchange_code_returns_unavailable_without_credentials() {
        if std::env::var("OAUTH2_GITHUB_CLIENT_ID").is_err() {
            match exchange_code(
                OAuth2Provider::GitHub,
                "https://localhost/callback",
                "fake-code",
            ) {
                OAuth2Result::ProviderUnavailable(msg) => {
                    assert!(msg.contains("credentials not configured"));
                }
                other => panic!("Expected ProviderUnavailable, got {other:?}"),
            }
        }
    }

    #[test]
    fn generate_auth_url_returns_error_without_credentials() {
        if std::env::var("OAUTH2_GITHUB_CLIENT_ID").is_err() {
            let result = generate_auth_url(
                OAuth2Provider::GitHub,
                "https://localhost/callback",
            );
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("credentials not configured"));
        }
    }
}
