//! Hosted FerrFlow bot OIDC exchange.
//!
//! When the user opts into `bot: true` on the GitHub Action, the composite
//! action simply forwards the relevant environment variables to this binary.
//! This module performs the full OIDC exchange in-process, so self-hosted
//! runners do not need Node.js (or any other runtime) installed.
//!
//! Flow:
//! 1. Read `ACTIONS_ID_TOKEN_REQUEST_URL` and `ACTIONS_ID_TOKEN_REQUEST_TOKEN`
//!    provided by the GitHub Actions runner (requires `permissions.id-token: write`).
//! 2. GET `{url}&audience={audience}` with the bearer request token to obtain
//!    a short-lived OIDC JWT from the runner.
//! 3. POST that JWT to the hosted bot service, which verifies it and returns
//!    a short-lived GitHub App installation token.
//! 4. Export that token into the process environment as both `GITHUB_TOKEN`
//!    and `FERRFLOW_TOKEN` so the rest of FerrFlow picks it up transparently.

use anyhow::{Context, Result, bail};

const DEFAULT_ENDPOINT: &str = "https://api.ferrlabs.com/api/v1/ferrflow/token";
const DEFAULT_AUDIENCE: &str = "ferrflow.ferrlabs.com";

/// Returns true when the `FERRFLOW_BOT` env var is set to a truthy value.
pub fn bot_mode_enabled() -> bool {
    match std::env::var("FERRFLOW_BOT") {
        Ok(value) => {
            let v = value.trim().to_ascii_lowercase();
            matches!(v.as_str(), "true" | "1")
        }
        Err(_) => false,
    }
}

pub struct BotTokenExchange {
    pub endpoint: String,
    pub audience: String,
}

impl Default for BotTokenExchange {
    fn default() -> Self {
        Self {
            endpoint: std::env::var("FERRFLOW_BOT_ENDPOINT")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| DEFAULT_ENDPOINT.to_string()),
            audience: std::env::var("FERRFLOW_BOT_AUDIENCE")
                .ok()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| DEFAULT_AUDIENCE.to_string()),
        }
    }
}

#[derive(Debug)]
pub struct IssuedToken {
    pub token: String,
    pub expires_at: String,
    pub repository: String,
}

#[derive(serde::Deserialize)]
struct IssuedTokenResponse {
    token: String,
    #[serde(default)]
    expires_at: String,
    #[serde(default)]
    repository: String,
}

#[derive(serde::Deserialize)]
struct OidcResponse {
    value: String,
}

impl BotTokenExchange {
    /// Runs the full OIDC exchange and returns a short-lived installation token.
    pub fn issue(&self) -> Result<IssuedToken> {
        let req_url = std::env::var("ACTIONS_ID_TOKEN_REQUEST_URL").map_err(|_| {
            anyhow::anyhow!(
                "bot mode requires `permissions: id-token: write` in your workflow — ACTIONS_ID_TOKEN_REQUEST_URL not set"
            )
        })?;
        let req_token = std::env::var("ACTIONS_ID_TOKEN_REQUEST_TOKEN").map_err(|_| {
            anyhow::anyhow!(
                "bot mode requires `permissions: id-token: write` in your workflow — ACTIONS_ID_TOKEN_REQUEST_TOKEN not set"
            )
        })?;

        // 1. Fetch the runner OIDC JWT, scoped to the FerrFlow audience.
        let separator = if req_url.contains('?') { '&' } else { '?' };
        let oidc_url = format!(
            "{req_url}{separator}audience={}",
            encode_query_component(&self.audience)
        );

        let oidc_body: OidcResponse = ureq::get(&oidc_url)
            .header("Authorization", &format!("Bearer {req_token}"))
            .header("Accept", "application/json")
            .header(
                "User-Agent",
                concat!("ferrflow/", env!("CARGO_PKG_VERSION")),
            )
            .call()
            .context("failed to request OIDC token from GitHub Actions runner")?
            .body_mut()
            .read_json()
            .context("OIDC response from runner was not valid JSON")?;

        if oidc_body.value.is_empty() {
            bail!("OIDC response from GitHub Actions runner was missing the `value` field");
        }

        // 2. Exchange the JWT with the FerrFlow hosted bot service.
        let payload = serde_json::json!({ "token": oidc_body.value });
        let mut response = match ureq::post(&self.endpoint)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header(
                "User-Agent",
                concat!("ferrflow/", env!("CARGO_PKG_VERSION")),
            )
            .send_json(payload)
        {
            Ok(r) => r,
            Err(ureq::Error::StatusCode(code)) => {
                return Err(map_status_error(code));
            }
            Err(err) => {
                bail!(
                    "FerrFlow hosted bot unavailable: {err}. Check https://status.ferrlabs.com or fall back to a PAT via `token:`."
                );
            }
        };

        let body: IssuedTokenResponse = response
            .body_mut()
            .read_json()
            .context("FerrFlow bot service response was not valid JSON")?;

        if body.token.is_empty() {
            bail!("FerrFlow bot service response did not contain a token");
        }

        Ok(IssuedToken {
            token: body.token,
            expires_at: body.expires_at,
            repository: body.repository,
        })
    }
}

fn map_status_error(code: u16) -> anyhow::Error {
    match code {
        401 => anyhow::anyhow!(
            "FerrFlow OIDC verification failed (401). The runner's OIDC token was rejected by the hosted bot service."
        ),
        404 => anyhow::anyhow!(
            "FerrFlow App not installed on this repository's owner. Install at https://github.com/apps/ferrflow"
        ),
        429 => anyhow::anyhow!(
            "FerrFlow hosted bot rate limit hit (429). Retry shortly or use `token:` with a PAT."
        ),
        500..=599 => anyhow::anyhow!(
            "FerrFlow hosted bot service unavailable ({code}). Check https://status.ferrlabs.com"
        ),
        _ => anyhow::anyhow!("FerrFlow hosted bot returned unexpected HTTP status {code}"),
    }
}

/// Minimal RFC 3986 query component encoder — enough for the audience string,
/// which is almost always a plain hostname. Avoids pulling in a URL crate.
fn encode_query_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        let safe = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~');
        if safe {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/// If `FERRFLOW_BOT` is enabled, perform the OIDC exchange and export the
/// resulting installation token into the process environment so the rest
/// of FerrFlow (forge, git push) picks it up via the normal lookup.
///
/// Safe to call more than once; the exchange only runs on the first call.
pub fn ensure_bot_token() -> Result<()> {
    if !bot_mode_enabled() {
        return Ok(());
    }

    // If a previous invocation already exchanged, don't do it again.
    static EXCHANGED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    if EXCHANGED.get().is_some() {
        return Ok(());
    }

    let exchange = BotTokenExchange::default();
    let issued = exchange
        .issue()
        .context("failed to obtain FerrFlow bot token")?;

    // SAFETY: set_var is marked unsafe in edition 2024. This is single-threaded
    // initialization at the top of a command, before any spawned threads read
    // these variables. Same pattern as the rest of FerrFlow's env handling
    // (see git.rs tests).
    unsafe {
        std::env::set_var("GITHUB_TOKEN", &issued.token);
        std::env::set_var("FERRFLOW_TOKEN", &issued.token);
    }

    // Mask the token for any downstream log sinks that honor GitHub's
    // `::add-mask::` workflow command.
    println!("::add-mask::{}", issued.token);

    let repo_note = if issued.repository.is_empty() {
        String::new()
    } else {
        format!(" on {}", issued.repository)
    };
    let expires_note = if issued.expires_at.is_empty() {
        String::new()
    } else {
        format!(" (expires at {})", issued.expires_at)
    };
    println!("Authenticated as ferrflow[bot]{repo_note}{expires_note}.");

    // actions/checkout (and similar CI primitives) install GITHUB_TOKEN
    // via two mechanisms that survive `persist-credentials: false`:
    //   1. an `http.https://github.com/.extraheader` directly in the local
    //      git config, and
    //   2. an `includeIf.gitdir:<repo>.path` entry pointing at a temp
    //      credentials config file that holds the same extraheader.
    //
    // Either one outranks the URL-embedded token FerrFlow installs via
    // `build_authenticated_url`, so every git push authenticates as
    // github-actions[bot] (the GITHUB_TOKEN identity) instead of
    // ferrflow[bot]. That blows up against branch rulesets where the
    // App identity is in the bypass list and github-actions[bot] is not.
    //
    // Strip both unconditionally on bot mode entry. We're inside a CI
    // checkout that's about to be torn down at the end of the job — there
    // is no risk of leaking these for later use.
    strip_cached_https_credentials();

    let _ = EXCHANGED.set(());
    Ok(())
}

/// Remove the `http.https://github.com/.extraheader` and any
/// `includeIf.gitdir:…` entries that `actions/checkout` (or equivalent)
/// installed in the current repo's local git config so they don't outrank
/// the App installation token FerrFlow injects via the remote URL on push.
///
/// Best-effort: failures (e.g. not in a repo, or no entries to remove)
/// are silently ignored — the worst case is that the existing behaviour
/// continues. Stays in scope only for explicit bot mode.
fn strip_cached_https_credentials() {
    let cwd = match std::env::current_dir() {
        Ok(p) => p,
        Err(_) => return,
    };
    strip_cached_https_credentials_in(&cwd);
}

/// Inner form that accepts an explicit working directory — the public
/// helper resolves it from the process's cwd. Split out so unit tests
/// can target a tempdir without racing other tests on the global cwd.
fn strip_cached_https_credentials_in(repo_dir: &std::path::Path) {
    let _ = std::process::Command::new("git")
        .args([
            "config",
            "--local",
            "--unset-all",
            "http.https://github.com/.extraheader",
        ])
        .current_dir(repo_dir)
        .status();

    // includeIf entries are keyed by the gitdir path, so we have to list
    // them first then unset each. The `--get-regexp` returns lines like
    // `includeIf.gitdir:/path/.path /path/to/credentials.config` — we
    // only need the key (first whitespace-delimited token).
    if let Ok(output) = std::process::Command::new("git")
        .args(["config", "--local", "--get-regexp", "^includeIf\\."])
        .current_dir(repo_dir)
        .output()
        && output.status.success()
    {
        let listing = String::from_utf8_lossy(&output.stdout);
        for line in listing.lines() {
            if let Some(key) = line.split_whitespace().next() {
                let _ = std::process::Command::new("git")
                    .args(["config", "--local", "--unset-all", key])
                    .current_dir(repo_dir)
                    .status();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce()>(vars: &[(&str, Option<&str>)], f: F) {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let previous: Vec<(String, Option<String>)> = vars
            .iter()
            .map(|(k, _)| ((*k).to_string(), std::env::var(*k).ok()))
            .collect();
        for (k, v) in vars {
            unsafe {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
        f();
        for (k, v) in previous {
            unsafe {
                match v {
                    Some(val) => std::env::set_var(&k, val),
                    None => std::env::remove_var(&k),
                }
            }
        }
    }

    #[test]
    fn bot_mode_detection() {
        with_env(&[("FERRFLOW_BOT", Some("true"))], || {
            assert!(bot_mode_enabled());
        });
        with_env(&[("FERRFLOW_BOT", Some("1"))], || {
            assert!(bot_mode_enabled());
        });
        with_env(&[("FERRFLOW_BOT", Some("TRUE"))], || {
            assert!(bot_mode_enabled());
        });
        with_env(&[("FERRFLOW_BOT", Some("false"))], || {
            assert!(!bot_mode_enabled());
        });
        with_env(&[("FERRFLOW_BOT", Some(""))], || {
            assert!(!bot_mode_enabled());
        });
        with_env(&[("FERRFLOW_BOT", None)], || {
            assert!(!bot_mode_enabled());
        });
    }

    #[test]
    fn defaults_use_hosted_endpoint_and_audience() {
        with_env(
            &[
                ("FERRFLOW_BOT_ENDPOINT", None),
                ("FERRFLOW_BOT_AUDIENCE", None),
            ],
            || {
                let ex = BotTokenExchange::default();
                assert_eq!(ex.endpoint, DEFAULT_ENDPOINT);
                assert_eq!(ex.audience, DEFAULT_AUDIENCE);
            },
        );
    }

    #[test]
    fn overrides_applied() {
        with_env(
            &[
                ("FERRFLOW_BOT_ENDPOINT", Some("https://example.test/t")),
                ("FERRFLOW_BOT_AUDIENCE", Some("aud.example.test")),
            ],
            || {
                let ex = BotTokenExchange::default();
                assert_eq!(ex.endpoint, "https://example.test/t");
                assert_eq!(ex.audience, "aud.example.test");
            },
        );
    }

    #[test]
    fn empty_overrides_fall_back_to_defaults() {
        with_env(
            &[
                ("FERRFLOW_BOT_ENDPOINT", Some("")),
                ("FERRFLOW_BOT_AUDIENCE", Some("")),
            ],
            || {
                let ex = BotTokenExchange::default();
                assert_eq!(ex.endpoint, DEFAULT_ENDPOINT);
                assert_eq!(ex.audience, DEFAULT_AUDIENCE);
            },
        );
    }

    #[test]
    fn issue_errors_when_runner_env_missing() {
        with_env(
            &[
                ("ACTIONS_ID_TOKEN_REQUEST_URL", None),
                ("ACTIONS_ID_TOKEN_REQUEST_TOKEN", None),
            ],
            || {
                let err = BotTokenExchange::default().issue().unwrap_err();
                let msg = err.to_string();
                assert!(
                    msg.contains("id-token: write"),
                    "expected id-token hint in error, got: {msg}"
                );
            },
        );
    }

    #[test]
    fn encode_query_component_leaves_safe_chars() {
        assert_eq!(
            encode_query_component("ferrflow.ferrlabs.com"),
            "ferrflow.ferrlabs.com"
        );
    }

    #[test]
    fn encode_query_component_escapes_unsafe() {
        assert_eq!(encode_query_component("a b&c=d"), "a%20b%26c%3Dd");
    }

    /// Reproduce what `actions/checkout` leaves behind (extraheader
    /// directly + an `includeIf.gitdir:` pointing at a temp credentials
    /// config), then verify that `strip_cached_https_credentials` removes
    /// both. The repo it runs against is the test's tempdir, scoped
    /// through `set_current_dir`.
    #[test]
    fn strip_cached_https_credentials_removes_extraheader_and_include_if() {
        // Don't run if git isn't on PATH (CI without git would skip — but
        // every CI we care about has it).
        if std::process::Command::new("git")
            .arg("--version")
            .status()
            .is_err()
        {
            return;
        }

        let tmp = tempfile::tempdir().unwrap();
        let repo_path = tmp.path();
        // Init a repo so we have a `.git/config` to mutate.
        let init = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(repo_path)
            .status()
            .unwrap();
        assert!(init.success());

        // Plant the two credentials shapes actions/checkout leaves behind.
        let cred_file = repo_path.join("creds.config");
        std::fs::write(&cred_file, "").unwrap();
        let set = |key: &str, val: &str| {
            assert!(
                std::process::Command::new("git")
                    .args(["config", "--local", key, val])
                    .current_dir(repo_path)
                    .status()
                    .unwrap()
                    .success(),
                "git config {key} failed"
            );
        };
        set(
            "http.https://github.com/.extraheader",
            "AUTHORIZATION: basic dGVzdA==",
        );
        set(
            &format!("includeIf.gitdir:{}.path", repo_path.join(".git").display()),
            cred_file.to_str().unwrap(),
        );

        // Sanity: both should be readable now.
        let read = |key: &str| -> Option<String> {
            let out = std::process::Command::new("git")
                .args(["config", "--local", "--get", key])
                .current_dir(repo_path)
                .output()
                .ok()?;
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        };
        assert!(read("http.https://github.com/.extraheader").is_some());
        let include_key = format!("includeIf.gitdir:{}.path", repo_path.join(".git").display());
        assert!(read(&include_key).is_some());

        // Use the path-taking inner so we don't race other tests on cwd.
        strip_cached_https_credentials_in(repo_path);

        assert!(
            read("http.https://github.com/.extraheader").is_none(),
            "extraheader should have been unset"
        );
        assert!(
            read(&include_key).is_none(),
            "includeIf.gitdir entry should have been unset"
        );
    }
}
