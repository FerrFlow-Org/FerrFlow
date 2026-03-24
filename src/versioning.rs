use crate::conventional_commits::BumpType;
use anyhow::Result;
use semver::Version;

pub fn bump_version(current: &str, bump: BumpType) -> Result<String> {
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
}
