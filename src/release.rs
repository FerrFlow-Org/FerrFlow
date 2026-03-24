// GitHub Releases — placeholder for v0.2.0
// Will use the `octocrab` crate to create releases via the GitHub API.

#[allow(dead_code)]
pub struct ReleaseOptions {
    pub token: String,
    pub repo: String, // "owner/repo"
    pub tag: String,
    pub name: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
}
