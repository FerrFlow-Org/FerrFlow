pub mod json;
pub mod toml_format;
pub mod xml;

use anyhow::Result;
use std::path::Path;

use crate::config::{FileFormat, VersionedFile};

pub trait VersionFile {
    fn read_version(&self, file_path: &Path) -> Result<String>;
    fn write_version(&self, file_path: &Path, version: &str) -> Result<()>;
}

pub fn get_handler(format: &FileFormat) -> Box<dyn VersionFile> {
    match format {
        FileFormat::Json => Box::new(json::JsonVersionFile),
        FileFormat::Toml => Box::new(toml_format::TomlVersionFile),
        FileFormat::Xml => Box::new(xml::XmlVersionFile),
    }
}

pub fn read_version(vf: &VersionedFile, repo_root: &Path) -> Result<String> {
    let path = repo_root.join(&vf.path);
    let handler = get_handler(&vf.format);
    handler.read_version(&path)
}

pub fn write_version(vf: &VersionedFile, repo_root: &Path, version: &str) -> Result<()> {
    let path = repo_root.join(&vf.path);
    let handler = get_handler(&vf.format);
    handler.write_version(&path, version)
}
