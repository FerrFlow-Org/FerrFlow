use crate::config::VersioningStrategy;
use crate::conventional_commits::BumpType;
use anyhow::Result;
use chrono::Local;
use semver::Version;

pub fn compute_next_version(
    current: &str,
    bump: BumpType,
    strategy: VersioningStrategy,
) -> Result<String> {
    match strategy {
        VersioningStrategy::Semver => bump_semver(current, bump),
        VersioningStrategy::Calver => calver_version("%Y.%m.%d"),
        VersioningStrategy::CalverShort => calver_version("short"),
        VersioningStrategy::CalverSeq => calver_seq_version(current),
        VersioningStrategy::Sequential => bump_sequential(current),
        VersioningStrategy::Zerover => bump_zerover(current, bump),
    }
}

fn bump_semver(current: &str, bump: BumpType) -> Result<String> {
    let mut v = Version::parse(current.trim_start_matches('v'))
        .map_err(|e| anyhow::anyhow!("Invalid semver '{}': {}", current, e))?;

    match bump {
        BumpType::Major => {
            v.major += 1;
            v.minor = 0;
            v.patch = 0;
        }
        BumpType::Minor => {
            v.minor += 1;
            v.patch = 0;
        }
        BumpType::Patch => {
            v.patch += 1;
        }
        BumpType::None => {}
    }

    Ok(v.to_string())
}

fn calver_version(format: &str) -> Result<String> {
    let now = Local::now();
    if format == "short" {
        Ok(format!(
            "{}.{}.{}",
            now.format("%y"),
            now.format("%-m"),
            now.format("%-d")
        ))
    } else {
        Ok(now.format("%Y.%-m.%-d").to_string())
    }
}

fn calver_seq_version(current: &str) -> Result<String> {
    let now = Local::now();
    let year_month = format!("{}.{}", now.format("%Y"), now.format("%-m"));

    // Parse current version to check if same year.month prefix
    let seq = if current.starts_with(&year_month) {
        // Same month — increment the sequence number
        let parts: Vec<&str> = current.splitn(3, '.').collect();
        if parts.len() == 3 {
            parts[2].parse::<u32>().unwrap_or(0) + 1
        } else {
            1
        }
    } else {
        1
    };

    Ok(format!("{year_month}.{seq}"))
}

fn bump_sequential(current: &str) -> Result<String> {
    let n: u64 = current.trim_start_matches('v').parse().unwrap_or_else(|_| {
        // Try parsing as semver and use patch as sequence
        Version::parse(current.trim_start_matches('v'))
            .map(|v| v.patch)
            .unwrap_or(0)
    });
    Ok((n + 1).to_string())
}

fn bump_zerover(current: &str, bump: BumpType) -> Result<String> {
    let mut v = Version::parse(current.trim_start_matches('v'))
        .map_err(|e| anyhow::anyhow!("Invalid semver '{}': {}", current, e))?;

    match bump {
        // Major bump becomes minor in zerover
        BumpType::Major => {
            v.minor += 1;
            v.patch = 0;
        }
        BumpType::Minor => {
            v.minor += 1;
            v.patch = 0;
        }
        BumpType::Patch => {
            v.patch += 1;
        }
        BumpType::None => {}
    }

    v.major = 0;
    Ok(v.to_string())
}

// Keep backward-compatible alias used by tests and other modules
pub fn bump_version(current: &str, bump: BumpType) -> Result<String> {
    bump_semver(current, bump)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bump_patch() {
        assert_eq!(bump_version("1.2.3", BumpType::Patch).unwrap(), "1.2.4");
    }

    #[test]
    fn test_bump_minor() {
        assert_eq!(bump_version("1.2.3", BumpType::Minor).unwrap(), "1.3.0");
    }

    #[test]
    fn test_bump_major() {
        assert_eq!(bump_version("1.2.3", BumpType::Major).unwrap(), "2.0.0");
    }

    #[test]
    fn test_bump_none() {
        assert_eq!(bump_version("1.2.3", BumpType::None).unwrap(), "1.2.3");
    }

    #[test]
    fn test_bump_with_v_prefix() {
        assert_eq!(bump_version("v1.2.3", BumpType::Patch).unwrap(), "1.2.4");
    }

    #[test]
    fn test_zerover_major_becomes_minor() {
        assert_eq!(bump_zerover("0.5.2", BumpType::Major).unwrap(), "0.6.0");
    }

    #[test]
    fn test_zerover_clamps_major() {
        assert_eq!(bump_zerover("0.9.0", BumpType::Major).unwrap(), "0.10.0");
    }

    #[test]
    fn test_zerover_patch() {
        assert_eq!(bump_zerover("0.5.2", BumpType::Patch).unwrap(), "0.5.3");
    }

    #[test]
    fn test_sequential() {
        assert_eq!(bump_sequential("41").unwrap(), "42");
    }

    #[test]
    fn test_sequential_from_zero() {
        assert_eq!(bump_sequential("0").unwrap(), "1");
    }

    #[test]
    fn test_calver_format() {
        let v = calver_version("%Y.%m.%d").unwrap();
        // Should have 3 dot-separated parts
        assert_eq!(v.split('.').count(), 3);
    }

    #[test]
    fn test_calver_short_format() {
        let v = calver_version("short").unwrap();
        assert_eq!(v.split('.').count(), 3);
        // Year part should be 2 digits
        let year: u32 = v.split('.').next().unwrap().parse().unwrap();
        assert!(year < 100);
    }

    #[test]
    fn test_calver_seq_new_month() {
        let v = calver_seq_version("2024.1.5").unwrap();
        let parts: Vec<&str> = v.split('.').collect();
        assert_eq!(parts.len(), 3);
        // Should be current year.month.1 (new month resets seq)
        assert_eq!(parts[2], "1");
    }

    #[test]
    fn test_calver_seq_same_month() {
        let now = chrono::Local::now();
        let current = format!("{}.{}.3", now.format("%Y"), now.format("%-m"));
        let v = calver_seq_version(&current).unwrap();
        assert!(v.ends_with(".4"));
    }

    #[test]
    fn test_compute_next_version_semver() {
        assert_eq!(
            compute_next_version("1.2.3", BumpType::Minor, VersioningStrategy::Semver).unwrap(),
            "1.3.0"
        );
    }

    #[test]
    fn test_compute_next_version_zerover() {
        assert_eq!(
            compute_next_version("0.5.2", BumpType::Major, VersioningStrategy::Zerover).unwrap(),
            "0.6.0"
        );
    }

    #[test]
    fn test_compute_next_version_sequential() {
        assert_eq!(
            compute_next_version("10", BumpType::Patch, VersioningStrategy::Sequential).unwrap(),
            "11"
        );
    }
}
