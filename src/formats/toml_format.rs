use super::VersionFile;
use anyhow::{Context, Result};
use std::path::Path;

pub struct TomlVersionFile;

impl VersionFile for TomlVersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))?;
        let doc: toml_edit::DocumentMut = content
            .parse()
            .with_context(|| format!("Invalid TOML in {}", file_path.display()))?;

        // Cargo.toml: [package] version
        if let Some(v) = doc
            .get("package")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Ok(v.to_string());
        }

        // pyproject.toml: [project] version
        if let Some(v) = doc
            .get("project")
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Ok(v.to_string());
        }

        // pyproject.toml (poetry): [tool.poetry] version
        if let Some(v) = doc
            .get("tool")
            .and_then(|t| t.get("poetry"))
            .and_then(|p| p.get("version"))
            .and_then(|v| v.as_str())
        {
            return Ok(v.to_string());
        }

        anyhow::bail!("No version found in {}", file_path.display())
    }

    fn write_version(&self, file_path: &Path, version: &str) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Cannot read {}", file_path.display()))?;
        let mut doc: toml_edit::DocumentMut = content.parse()?;

        let mut written = false;

        if let Some(pkg) = doc.get_mut("package")
            && let Some(v) = pkg.get_mut("version")
            && v.is_str()
        {
            *v = toml_edit::value(version);
            written = true;
        }

        if !written
            && let Some(proj) = doc.get_mut("project")
            && let Some(v) = proj.get_mut("version")
        {
            *v = toml_edit::value(version);
            written = true;
        }

        if !written
            && let Some(tool) = doc.get_mut("tool")
            && let Some(poetry) = tool.get_mut("poetry")
            && let Some(v) = poetry.get_mut("version")
        {
            *v = toml_edit::value(version);
            written = true;
        }

        if !written {
            anyhow::bail!(
                "Could not find version field to update in {}",
                file_path.display()
            );
        }

        std::fs::write(file_path, doc.to_string())?;
        Ok(())
    }
}
