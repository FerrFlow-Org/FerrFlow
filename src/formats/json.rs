use super::VersionFile;
use anyhow::{Context, Result};
use std::path::Path;

pub struct JsonVersionFile;

impl VersionFile for JsonVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))?;
        let v: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| format!("Invalid JSON in {}", file_path.display()))?;
        v["version"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("No 'version' field in {}", file_path.display()))
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))?;
        let mut v: serde_json::Value = serde_json::from_str(&content)?;
        v["version"] = serde_json::Value::String(version.to_string());
        let new_content = serde_json::to_string_pretty(&v)? + "\n";
        std::fs::write(file_path, new_content)?;
        Ok(())
    }
}
