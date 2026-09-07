use anyhow::Result;
use colored::Colorize;
use std::time::Duration;

use crate::error_code;

pub(super) fn retry_transient<F>(label: &str, mut op: F) -> Result<()>
where
    F: FnMut() -> Result<()>,
{
    const MAX_ATTEMPTS: u32 = 4;
    let mut delay = Duration::from_secs(1);

    for attempt in 1..=MAX_ATTEMPTS {
        match op() {
            Ok(()) => {
                if attempt > 1 {
                    tracing::info!(
                        "{}",
                        format!("  ✓ {label} succeeded on attempt {attempt}/{MAX_ATTEMPTS}")
                            .green()
                    );
                }
                return Ok(());
            }
            Err(err) => {
                let transient = is_transient_git_error(&err);
                if !transient || attempt == MAX_ATTEMPTS {
                    return Err(err);
                }
                tracing::warn!(
                    "{}",
                    format!(
                        "  ⚠ {label} attempt {attempt}/{MAX_ATTEMPTS} failed (transient): {err}; \
                         retrying in {}s",
                        delay.as_secs()
                    )
                    .yellow()
                );
                std::thread::sleep(delay);
                delay = delay.saturating_mul(2);
            }
        }
    }
    unreachable!("the final attempt returns rather than falling out of the loop")
}

/// Failures that never get better by trying again. Checked before anything
/// else: git embeds the remote URL, the refspec and the tag name in its error
/// text, so a terminal failure on `acme/network-api` or a push of `v1.503.0`
/// used to match a transient needle and burn the whole retry budget first.
const TERMINAL: &[&str] = &[
    "non-fast-forward",
    "branch protection",
    "rejected by remote",
    "authentication failed",
    "permission denied",
    "repository not found",
];

/// Phrases naming a failure mode, rather than bare tokens. `network`, `ssl`,
/// `tls` and `connection` all appear in repository names, so on their own they
/// classify the remote rather than the error.
const TRANSIENT: &[&str] = &[
    "connection refused",
    "connection reset",
    "connection closed",
    "connection timed out",
    "failed to connect",
    "could not resolve host",
    "could not resolve proxy",
    "temporarily unavailable",
    "network is unreachable",
    "network is down",
    "network error",
    "timed out",
    "broken pipe",
    "rst_stream",
    "remote end hung up",
    "early eof",
    "ssl handshake",
    "ssl connect error",
    "ssl_read",
    "ssl_write",
    "sslv3",
    "tls handshake",
    "gnutls_handshake",
    "certificate verify failed",
    "bad gateway",
    "service unavailable",
    "gateway timeout",
    "internal server error",
    "secondary rate limit",
    "rate limit exceeded",
    "fatal error in commit_refs",
    "object is no commit object",
    "no commit object",
    "class=invalid",
    "object not found",
    "odb",
];

/// Status codes worth retrying, matched only where git or curl actually reports
/// one. A bare `503` matches the tag `v1.503.0`.
const TRANSIENT_STATUS: &[&str] = &["502", "503", "504"];

fn mentions_http_status(chain: &str, code: &str) -> bool {
    [
        "error: ",
        "http ",
        "http/1.1 ",
        "http/2 ",
        "status ",
        "status code ",
        "code ",
    ]
    .iter()
    .any(|prefix| chain.contains(&format!("{prefix}{code}")))
}

pub(super) fn is_transient_git_error(err: &anyhow::Error) -> bool {
    let chain = err
        .chain()
        .map(|e| e.to_string().to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");

    if TERMINAL.iter().any(|phrase| chain.contains(phrase)) {
        return false;
    }
    if TRANSIENT.iter().any(|phrase| chain.contains(phrase)) {
        return true;
    }
    TRANSIENT_STATUS
        .iter()
        .any(|code| mentions_http_status(&chain, code))
}

/// Whether the push failed because the remote moved under us, which the release
/// flow answers by regenerating the release commit against the new tip.
///
/// This deliberately does not match `GIT_PUSH_BRANCH` or `GIT_PUSH_TAGS`. Those
/// are the generic "push failed" codes carried by every push error, transient
/// and permanent alike, so matching them reported a stale branch for causes that
/// were nothing of the sort and burned the regenerate attempts on errors no
/// amount of regenerating could fix.
pub fn is_push_rejected_error(err: &anyhow::Error) -> bool {
    let rejected_code = error_code::GIT_PUSH_REJECTED.to_string();
    err.chain().any(|cause| {
        let raw = cause.to_string();
        if raw == rejected_code {
            return true;
        }
        let msg = raw.to_lowercase();
        msg.contains("rebase conflict")
            || msg.contains("push declined due to repository rule")
            || msg.contains("non-fast-forward")
            || msg.contains("non-fastforward")
            || msg.contains("not fast forward")
            // git says "fetch first" when the remote advanced and we have not
            // fetched since, which is the usual shape in CI, and "stale info"
            // when a --force-with-lease expectation is out of date.
            || msg.contains("fetch first")
            || msg.contains("stale info")
            || msg.contains("already exist on remote pointing to a different commit")
            || msg.contains("expected branch to point to")
    })
}
