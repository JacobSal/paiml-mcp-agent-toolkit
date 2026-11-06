//! Streamable-HTTP MCP transport (EV-6, #999).
//!
//! Serves the same tool surface as the stdio server — `build_server` is shared,
//! so "which tools does pmat advertise" keeps one answer — over pmcp's
//! `StreamableHttpServer`.
//!
//! # Why streamable HTTP specifically
//!
//! The Google Antigravity Agent registers remote MCP servers as
//! `{"type": "mcp_server", "name": ..., "url": ...}` and supports **streamable
//! HTTP only** — SSE is explicitly unsupported — so stdio cannot reach it. Its
//! egress proxy also blocks loopback, so the endpoint has to be a reachable
//! allowlisted host rather than `localhost`.
//!
//! # It fails closed
//!
//! [`serve`] REFUSES TO START without a token. That is deliberate and it is the
//! whole point of the module: pmcp's HTTP layer calls
//! `server.get_auth_provider()` and, when there is none, falls through to
//! `extract_auth_from_proxy_headers` and returns `Ok(None)` — i.e. it serves
//! every request. A naive wiring would satisfy the sentence "unauthenticated
//! request → 401" in a ticket while doing the opposite in production, because
//! nothing would ever be unauthenticated. Requiring the token at construction
//! makes that state unreachable rather than merely discouraged.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{bail, Result};
use pmcp::server::auth::{AuthContext, AuthProvider};

/// Environment variable carrying the bearer token.
///
/// A token belongs in the environment, not in a flag: an argument is visible in
/// `ps` and lands in shell history. On Antigravity the bearer is injected at the
/// egress proxy via `network.allowlist[].transform`, so it never sits inside the
/// sandbox at all — that is the sovereign-safe shape and this variable is the
/// server's half of it.
pub const TOKEN_ENV: &str = "PMAT_MCP_HTTP_TOKEN";

/// Shortest token this will accept.
///
/// Not security theatre: the failure mode being prevented is a deployment that
/// sets `PMAT_MCP_HTTP_TOKEN=x`, passes its smoke test, and is effectively open.
///
/// Public so the help text and the generated-token path state the SAME floor
/// this constructor enforces. `serve --help` used to describe the minimum in
/// prose, which is how a documented number drifts from an enforced one.
pub const MIN_TOKEN_LEN: usize = 16;

/// Bearer-token auth over a single shared secret.
#[derive(Clone)]
pub struct BearerToken {
    token: Arc<String>,
}

impl BearerToken {
    /// Reject a token too short to be worth having.
    pub fn new(token: impl Into<String>) -> Result<Self> {
        let token = token.into();
        if token.len() < MIN_TOKEN_LEN {
            bail!(
                "{TOKEN_ENV} must be at least {MIN_TOKEN_LEN} characters; got {}",
                token.len()
            );
        }
        Ok(Self {
            token: Arc::new(token),
        })
    }

    /// Read the token from the environment.
    pub fn from_env() -> Result<Self> {
        match std::env::var(TOKEN_ENV) {
            Ok(t) => Self::new(t),
            // Exit code DECLARED, not inferred. This message used to reach
            // `ExitCode::ConfigurationError` (4) because the substring "config"
            // appears in "no auth provider is configured" — a clause about
            // pmcp's behaviour, in a message about a missing token. The help at
            // commands_enum/definition.rs:933 documents exit 4 as a contract,
            // so the code is kept and now stated here, where it cannot be
            // changed by editing prose.
            Err(_) => Err(crate::cli_exit::configuration_error(anyhow::anyhow!(
                "{TOKEN_ENV} is not set. `pmat serve` will not start an unauthenticated MCP \
                 endpoint: pmcp serves every request when no auth provider is configured, so \
                 starting without a token would publish the full tool surface to anyone who \
                 can reach the port."
            ))),
        }
    }

    /// Constant-time comparison.
    ///
    /// `==` on `String` short-circuits at the first differing byte, which leaks
    /// a matching prefix to anyone who can time the response. The cost here is
    /// nil and the alternative is a real, if unglamorous, oracle.
    fn matches(&self, candidate: &str) -> bool {
        let expected = self.token.as_bytes();
        let got = candidate.as_bytes();
        if expected.len() != got.len() {
            return false;
        }
        let mut diff = 0u8;
        for (a, b) in expected.iter().zip(got.iter()) {
            diff |= a ^ b;
        }
        diff == 0
    }
}

/// Redacted on purpose. `#[derive(Debug)]` would print the shared secret into
/// any log line, panic message or `expect` that formats this value — and the
/// test below formats it.
impl std::fmt::Debug for BearerToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BearerToken")
            .field("token", &"<redacted>")
            .finish()
    }
}

#[async_trait::async_trait]
impl AuthProvider for BearerToken {
    async fn validate_request(
        &self,
        authorization_header: Option<&str>,
    ) -> pmcp::error::Result<Option<AuthContext>> {
        // `None` here means "no credentials", and pmcp turns a validation error
        // into 401. Returning Ok(None) instead would be a pass.
        let header = authorization_header.ok_or_else(|| {
            pmcp::error::Error::Internal("missing Authorization header".to_string())
        })?;
        let presented = header.strip_prefix("Bearer ").ok_or_else(|| {
            pmcp::error::Error::Internal("Authorization header is not a Bearer token".to_string())
        })?;
        if !self.matches(presented.trim()) {
            return Err(pmcp::error::Error::Internal(
                "invalid bearer token".to_string(),
            ));
        }
        Ok(Some(AuthContext {
            subject: "pmat-mcp-http".to_string(),
            scopes: vec![],
            ..AuthContext::default()
        }))
    }

    fn auth_scheme(&self) -> &'static str {
        "Bearer"
    }

    fn is_required(&self) -> bool {
        true
    }
}

/// Environment variable listing extra origins the DNS rebinding guard accepts.
///
/// pmcp answers 403 to any `Host` other than `localhost`, `127.0.0.1` or
/// `[::1]`, which also refuses a legitimate caller on a private network, such
/// as an MCP hub reaching this server by its container name. The value is a
/// comma-separated list like `http://pmat-mcp:8765`. It adds to the localhost
/// defaults, and the bearer token is still required.
pub const ALLOWED_ORIGINS_ENV: &str = "PMAT_MCP_ALLOWED_ORIGINS";

/// Parse a comma-separated origin list, ignoring blank entries.
///
/// Strict on purpose. pmcp silently drops an entry with no scheme, so a bare
/// `pmat-mcp` would start cleanly and go on answering 403. There is no wildcard
/// either: a deployment that wants the guard off should have to say so in code.
fn parse_allowed_origins(raw: &str) -> Result<Vec<String>> {
    raw.split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            if is_origin(entry) {
                Ok(entry.to_string())
            } else {
                bail!(
                    "{ALLOWED_ORIGINS_ENV}: `{entry}` is not an origin; expected \
                     http://host[:port] or https://host[:port]"
                )
            }
        })
        .collect()
}

/// Whether `entry` is a scheme plus a host, with no path and no wildcard.
fn is_origin(entry: &str) -> bool {
    let authority = entry
        .strip_prefix("https://")
        .or_else(|| entry.strip_prefix("http://"));
    authority.is_some_and(|a| {
        !a.is_empty()
            && !a.starts_with(':')
            && a.chars()
                .all(|c| c.is_ascii_alphanumeric() || "-._:[]".contains(c))
    })
}

/// Read [`ALLOWED_ORIGINS_ENV`]. Unset means no extra origins.
pub fn allowed_origins_from_env() -> Result<Vec<String>> {
    match std::env::var(ALLOWED_ORIGINS_ENV) {
        Ok(raw) => parse_allowed_origins(&raw).map_err(crate::cli_exit::configuration_error),
        Err(std::env::VarError::NotPresent) => Ok(Vec::new()),
        Err(e) => Err(crate::cli_exit::configuration_error(anyhow::anyhow!(
            "{ALLOWED_ORIGINS_ENV}: {e}"
        ))),
    }
}

/// Start the streamable-HTTP MCP endpoint. Returns the bound address and the
/// server task.
///
/// Stateless by construction (`session_id_generator: None`): MCP revision
/// 2026-07-28 removes the initialize/initialized handshake and
/// `Mcp-Session-Id` entirely, and server-held session state is the pattern it
/// dropped. Starting stateless means the transport does not have to be
/// re-architected when pmat moves to that revision.
///
/// `extra_origins` extends the DNS rebinding guard's localhost defaults. When
/// it is empty the config stays `None`: `AllowedOrigins::explicit` on an empty
/// list would mean `any()`, which turns the guard off.
#[cfg(feature = "mcp-http")]
pub async fn serve(
    addr: SocketAddr,
    auth: BearerToken,
    extra_origins: Vec<String>,
) -> Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
    use pmcp::server::streamable_http_server::{StreamableHttpServer, StreamableHttpServerConfig};
    use pmcp::server::tower_layers::AllowedOrigins;

    let provider: Arc<dyn AuthProvider> = Arc::new(auth);
    let server =
        crate::mcp_pmcp::simple_unified_server::SimpleUnifiedServer::build_server(Some(provider))
            .map_err(|e| anyhow::anyhow!("building the MCP tool surface failed: {e}"))?;

    let allowed_origins = (!extra_origins.is_empty()).then(|| {
        let mut origins = AllowedOrigins::localhost().origins().to_vec();
        origins.extend(extra_origins);
        AllowedOrigins::explicit(origins)
    });
    let config = StreamableHttpServerConfig {
        session_id_generator: None, // stateless
        enable_json_response: true,
        allowed_origins,
        ..Default::default()
    };
    let http =
        StreamableHttpServer::with_config(addr, Arc::new(tokio::sync::Mutex::new(server)), config);
    http.start()
        .await
        .map_err(|e| anyhow::anyhow!("starting the streamable-HTTP MCP server failed: {e}"))
}

/// The build without the feature: say so rather than pretend.
#[cfg(not(feature = "mcp-http"))]
pub async fn serve(
    _addr: SocketAddr,
    _auth: BearerToken,
    _extra_origins: Vec<String>,
) -> Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
    bail!(
        "this pmat was built without the `mcp-http` feature, so the streamable-HTTP MCP \
         transport is not compiled in. Rebuild with `--features mcp-http`."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_token_is_refused() {
        assert!(
            BearerToken::new("tooshort").is_err(),
            "a token short enough to guess must not be accepted"
        );
        assert!(BearerToken::new("0123456789abcdef").is_ok());
    }

    /// The invariant: no token, no server. Asserted because the failure mode is
    /// silent — pmcp serves every request when no auth provider is configured,
    /// so a `serve` that started without one would look healthy and be open.
    #[test]
    fn without_a_token_the_server_refuses_to_start() {
        let saved = std::env::var(TOKEN_ENV).ok();
        std::env::remove_var(TOKEN_ENV);
        let got = BearerToken::from_env();
        if let Some(v) = saved {
            std::env::set_var(TOKEN_ENV, v);
        }
        let err = got.expect_err("must refuse without a token").to_string();
        assert!(
            err.contains("will not start an unauthenticated MCP endpoint"),
            "the error must say why, got: {err}"
        );
    }

    #[tokio::test]
    async fn a_request_without_credentials_is_rejected() {
        let auth = BearerToken::new("0123456789abcdef0123").expect("token");
        assert!(
            auth.validate_request(None).await.is_err(),
            "no Authorization header must not authenticate"
        );
        assert!(
            auth.validate_request(Some("Basic 0123456789abcdef0123"))
                .await
                .is_err(),
            "a non-Bearer scheme must not authenticate"
        );
        assert!(
            auth.validate_request(Some("Bearer wrongwrongwrongwrong"))
                .await
                .is_err(),
            "a wrong token must not authenticate"
        );
        assert!(
            auth.validate_request(Some("Bearer 0123456789abcdef0123"))
                .await
                .expect("valid token")
                .is_some(),
            "the correct token must authenticate"
        );
    }

    /// A prefix match must not pass, and must not be distinguishable by length.
    #[test]
    fn comparison_is_length_checked_and_constant_time() {
        let auth = BearerToken::new("0123456789abcdef0123").expect("token");
        assert!(
            !auth.matches("0123456789abcdef012"),
            "prefix must not match"
        );
        assert!(
            !auth.matches("0123456789abcdef01234"),
            "suffix must not match"
        );
        assert!(auth.matches("0123456789abcdef0123"));
    }

    #[test]
    fn allowed_origins_are_split_and_trimmed() {
        let got = parse_allowed_origins(" http://pmat-mcp:8765 ,,https://[::2]:9000,")
            .expect("valid list");
        assert_eq!(got, ["http://pmat-mcp:8765", "https://[::2]:9000"]);
    }

    /// An empty list must stay empty rather than erroring, because `serve` is
    /// what keeps it away from `AllowedOrigins::explicit`, where it means `any()`.
    #[test]
    fn a_blank_allowed_origins_value_adds_nothing() {
        assert!(parse_allowed_origins("").expect("blank").is_empty());
        assert!(parse_allowed_origins(" , ").expect("blank").is_empty());
    }

    #[test]
    fn a_malformed_allowed_origin_is_refused() {
        for bad in [
            "pmat-mcp",
            "pmat-mcp:8765",
            "*",
            "http://*",
            "http://",
            "http://:8765",
            "http://pmat-mcp:8765/path",
            "ftp://pmat-mcp",
        ] {
            let err = parse_allowed_origins(bad).expect_err(bad).to_string();
            assert!(err.contains(ALLOWED_ORIGINS_ENV), "got: {err}");
        }
    }

    #[test]
    fn auth_is_required_and_advertises_bearer() {
        let auth = BearerToken::new("0123456789abcdef0123").expect("token");
        assert!(auth.is_required(), "optional auth is not auth");
        assert_eq!(auth.auth_scheme(), "Bearer");
    }
}

#[cfg(all(test, feature = "mcp-http"))]
mod http_e2e_tests {
    //! The invariant EV-6 exists for, asserted against a REAL bound socket
    //! rather than against the provider in isolation: pmcp only consults the
    //! auth provider if one is wired, and the whole hazard is a wiring that
    //! looks right and serves open.
    use super::*;

    async fn start_with(
        extra_origins: Vec<String>,
    ) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        let auth = BearerToken::new("0123456789abcdef0123").expect("token");
        serve("127.0.0.1:0".parse().expect("addr"), auth, extra_origins)
            .await
            .expect("server starts")
    }

    async fn start() -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        start_with(Vec::new()).await
    }

    /// Status of a `tools/list` sent with this `Host` and optional token.
    async fn status_for(addr: std::net::SocketAddr, host: &str, token: Option<&str>) -> u16 {
        let mut request = reqwest::Client::new()
            .post(format!("http://{addr}/"))
            .header("Host", host)
            .header("Accept", "application/json, text/event-stream")
            .json(&serde_json::json!({
                "jsonrpc":"2.0","id":1,"method":"tools/list","params":{}
            }));
        if let Some(token) = token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }
        request.send().await.expect("request").status().as_u16()
    }

    const TOKEN: Option<&str> = Some("0123456789abcdef0123");

    #[tokio::test]
    async fn without_extra_origins_a_foreign_host_is_refused() {
        let (addr, handle) = start().await;
        assert_eq!(status_for(addr, "pmat-mcp:8765", TOKEN).await, 403);
        assert_eq!(status_for(addr, "localhost:8765", TOKEN).await, 200);
        handle.abort();
    }

    /// The setting adds one host. It must not drop the localhost defaults, open
    /// the guard to other hosts, or stand in for the token.
    #[tokio::test]
    async fn an_extra_origin_admits_only_that_host_and_keeps_auth() {
        let (addr, handle) = start_with(vec!["http://pmat-mcp:8765".to_string()]).await;
        assert_eq!(status_for(addr, "pmat-mcp:8765", TOKEN).await, 200);
        assert_eq!(status_for(addr, "localhost:8765", TOKEN).await, 200);
        assert_eq!(status_for(addr, "127.0.0.1:8765", TOKEN).await, 200);
        assert_eq!(status_for(addr, "evil.example:8765", TOKEN).await, 403);
        assert_eq!(status_for(addr, "pmat-mcp:8765", None).await, 401);
        handle.abort();
    }

    #[tokio::test]
    async fn unauthenticated_request_is_401_and_a_valid_token_is_not() {
        let (addr, handle) = start().await;
        // pmcp mounts the MCP endpoint at the ROOT of the server it starts
        // (`build_mcp_router` routes POST/GET/DELETE on "/"), not at /mcp. An
        // earlier version of this test used /mcp, got 404, and would have
        // "passed" a 401 assertion against a path that does not exist.
        let url = format!("http://{addr}/");
        let client = reqwest::Client::new();
        // Streamable HTTP negotiates content before it authenticates: without
        // this the server answers 406 and the 401 assertion below would be
        // testing content negotiation, not auth.
        const ACCEPT: &str = "application/json, text/event-stream";
        let body = serde_json::json!({
            "jsonrpc":"2.0","id":1,"method":"tools/list","params":{}
        });

        let no_auth = client
            .post(&url)
            .header("Accept", ACCEPT)
            .json(&body)
            .send()
            .await
            .expect("request");
        assert_eq!(
            no_auth.status().as_u16(),
            401,
            "an unauthenticated request must be refused, not served"
        );

        let bad = client
            .post(&url)
            .header("Accept", ACCEPT)
            .header("Authorization", "Bearer wrongwrongwrongwrong")
            .json(&body)
            .send()
            .await
            .expect("request");
        assert_eq!(bad.status().as_u16(), 401, "a wrong token must be refused");

        let good = client
            .post(&url)
            .header("Accept", ACCEPT)
            .header("Authorization", "Bearer 0123456789abcdef0123")
            .json(&body)
            .send()
            .await
            .expect("request");
        assert!(
            good.status().is_success(),
            "the correct token must be served, got {}",
            good.status()
        );
        handle.abort();
    }
}
