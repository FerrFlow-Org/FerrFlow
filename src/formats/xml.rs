use super::VersionFile;
use anyhow::{Context, Result};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

pub struct XmlVersionFile;

static VERSION_RE: OnceLock<Regex> = OnceLock::new();

fn version_re() -> &'static Regex {
    VERSION_RE.get_or_init(|| Regex::new(r"<version>([^<]+)</version>").unwrap())
}

impl VersionFile for XmlVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))?;

        version_re()
            .captures(&content)
            .map(|c| c[1].trim().to_string())
            .ok_or_else(|| anyhow::anyhow!("No <version> tag found in {}", file_path.display()))
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))?;

        let mut count = 0;
        let new_content = version_re().replace(&content, |_: &regex::Captures| {
            count += 1;
            format!("<version>{version}</version>")
        });

        if count == 0 {
            anyhow::bail!(
                "No <version> tag found to update in {}",
                file_path.display()
            );
        }

        std::fs::write(file_path, new_content.as_ref())?;
        Ok(())
    }
}
