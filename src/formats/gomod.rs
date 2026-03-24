use super::VersionFile;
use anyhow::{Context, Result};
use std::path::Path;

pub struct GoModVersionFile;

impl VersionFile for GoModVersionFile {
    fn read_version(&self, _file_path: &Path) -> Result<String> {
        let output = std::process::Command::new("git")
            .args(["describe", "--tags", "--match", "v*", "--abbrev=0"])
            .output()
            .context("Failed to run git describe")?;

        if !output.status.success() {
            anyhow::bail!(
                "No git tag matching 'v*' found. Create an initial tag first (e.g. git tag v0.1.0)."
            );
        }

        let tag = String::from_utf8_lossy(&output.stdout);
        let version = tag.trim().trim_start_matches('v');
        Ok(version.to_string())
    }

    fn write_version(&self, _file_path: &Path, _version: &str) -> Result<()> {
        // Go modules are versioned via git tags only — no file to update.
        Ok(())
    }
}
