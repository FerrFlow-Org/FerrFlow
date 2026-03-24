use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum BumpType {
    None,
    Patch,
    Minor,
    Major,
}

impl std::fmt::Display for BumpType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BumpType::None => write!(f, "none"),
            BumpType::Patch => write!(f, "patch"),
            BumpType::Minor => write!(f, "minor"),
            BumpType::Major => write!(f, "major"),
        }
    }
}

static BREAKING_RE: OnceLock<Regex> = OnceLock::new();
static FEAT_RE: OnceLock<Regex> = OnceLock::new();
static PATCH_RE: OnceLock<Regex> = OnceLock::new();

fn breaking_re() -> &'static Regex {
    BREAKING_RE.get_or_init(|| {
        Regex::new(r"(?m)^(feat|fix|refactor|perf|build|chore|docs|style|test|ci)(\(.+\))?!:|^BREAKING CHANGE").unwrap()
    })
}

fn feat_re() -> &'static Regex {
    FEAT_RE.get_or_init(|| Regex::new(r"(?m)^feat(\(.+\))?:").unwrap())
}

fn patch_re() -> &'static Regex {
    PATCH_RE.get_or_init(|| Regex::new(r"(?m)^(fix|perf|refactor)(\(.+\))?:").unwrap())
}

pub fn determine_bump(message: &str) -> BumpType {
    if breaking_re().is_match(message) {
        return BumpType::Major;
    }
    if feat_re().is_match(message) {
        return BumpType::Minor;
    }
    if patch_re().is_match(message) {
        return BumpType::Patch;
    }
    BumpType::None
}

pub fn parse_subject(message: &str) -> &str {
    message.lines().next().unwrap_or("").trim()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch() {
        assert_eq!(determine_bump("fix: correct typo"), BumpType::Patch);
        assert_eq!(determine_bump("perf: faster query"), BumpType::Patch);
        assert_eq!(determine_bump("refactor: clean up"), BumpType::Patch);
    }

    #[test]
    fn test_minor() {
        assert_eq!(determine_bump("feat: add login"), BumpType::Minor);
        assert_eq!(determine_bump("feat(auth): add JWT"), BumpType::Minor);
    }

    #[test]
    fn test_major() {
        assert_eq!(determine_bump("feat!: breaking change"), BumpType::Major);
        assert_eq!(
            determine_bump("fix(api)!: remove endpoint"),
            BumpType::Major
        );
        assert_eq!(
            determine_bump("BREAKING CHANGE: removed X"),
            BumpType::Major
        );
    }

    #[test]
    fn test_none() {
        assert_eq!(determine_bump("chore: update deps"), BumpType::None);
        assert_eq!(determine_bump("docs: update readme"), BumpType::None);
        assert_eq!(determine_bump("ci: fix pipeline"), BumpType::None);
    }
}
