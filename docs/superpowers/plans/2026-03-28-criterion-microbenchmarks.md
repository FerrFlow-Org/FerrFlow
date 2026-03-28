# Criterion Micro-Benchmarks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Criterion micro-benchmarks for internal subsystems that run on every PR, with automated comparison posted as a PR comment via `github-action-benchmark`.

**Architecture:** Create a `src/lib.rs` to expose internal modules to benchmarks (the crate is currently binary-only). Write a single `benches/ferrflow_benchmarks.rs` with 4 benchmark groups. Add a `micro-bench` CI job that runs on PRs (compare + comment) and on main pushes (upload baseline artifact).

**Tech Stack:** Criterion 0.5, github-action-benchmark v1, GitHub Actions artifacts

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `Cargo.toml` | Modify | Add `criterion` dev-dependency, `[[bench]]` section, `[lib]` section |
| `src/lib.rs` | Create | Re-export modules needed by benchmarks |
| `src/main.rs` | Modify | Import from lib instead of declaring modules directly |
| `benches/ferrflow_benchmarks.rs` | Create | All 4 benchmark groups |
| `.github/workflows/ci.yml` | Modify | Add `micro-bench` job |

---

### Task 1: Add lib target and Criterion dependency

**Files:**
- Modify: `Cargo.toml`
- Create: `src/lib.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add lib section and criterion to Cargo.toml**

Add after line 15 (`path = "src/main.rs"`):

```toml
[lib]
name = "ferrflow"
path = "src/lib.rs"
```

Add to `[dev-dependencies]`:

```toml
criterion = { version = "0.5", features = ["html_reports"] }
```

Add at the end of the file:

```toml
[[bench]]
name = "ferrflow_benchmarks"
harness = false
```

- [ ] **Step 2: Create src/lib.rs**

Create `src/lib.rs` that re-exports the modules benchmarks need:

```rust
pub mod changelog;
pub mod config;
pub mod conventional_commits;
pub mod formats;
pub mod git;
```

- [ ] **Step 3: Update src/main.rs to use lib re-exports**

Replace the module declarations at the top of `src/main.rs`:

```rust
mod cli;
mod monorepo;
mod query;
mod release;
mod status;
mod telemetry;
mod versioning;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
```

The modules that are now in `lib.rs` (`changelog`, `config`, `conventional_commits`, `formats`, `git`) are accessed via the lib crate automatically. The remaining modules (`cli`, `monorepo`, `query`, `release`, `status`, `telemetry`, `versioning`) stay as `mod` in main.rs because they're binary-only (they depend on CLI types or are not needed by benchmarks).

Note: modules in `main.rs` can use lib modules via `use ferrflow::config::Config;` etc., but since they already use `crate::config::Config`, both the `mod` declaration in main.rs and the `pub mod` in lib.rs would conflict. Instead, keep the modules that main.rs needs as `mod` in main.rs only, and put only the benchmark-facing modules in lib.rs. The modules that are in both would need to be removed from main.rs.

Actually, the simpler approach: keep all `mod` declarations in `main.rs` as they are, and in `lib.rs` just re-declare the same modules. Rust allows this because `main.rs` and `lib.rs` are separate crate roots. Each will compile independently.

So `src/main.rs` stays exactly as it is -- no changes needed:

```rust
mod changelog;
mod cli;
mod config;
mod conventional_commits;
mod formats;
mod git;
mod monorepo;
mod query;
mod release;
mod status;
mod telemetry;
mod versioning;

use anyhow::Result;
use clap::Parser;
use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.run()
}
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo build
cargo test
```

Expected: both pass. The lib crate and binary crate compile independently.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/lib.rs
git commit -m "chore: add lib target and criterion dependency for micro-benchmarks"
```

---

### Task 2: Write commit_parsing benchmarks

**Files:**
- Create: `benches/ferrflow_benchmarks.rs`

- [ ] **Step 1: Create the benchmark file with commit_parsing group**

Create `benches/ferrflow_benchmarks.rs`:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ferrflow::conventional_commits::determine_bump;

fn generate_commit_messages(count: usize) -> Vec<String> {
    let types = ["feat", "fix", "refactor", "perf", "chore", "docs", "ci", "test"];
    let scopes = ["api", "auth", "db", "cache", "config"];
    let mut messages = Vec::with_capacity(count);
    for i in 0..count {
        let t = types[i % types.len()];
        let s = scopes[i % scopes.len()];
        let breaking = if i % 20 == 0 { "!" } else { "" };
        messages.push(format!("{t}({s}){breaking}: change number {i}"));
    }
    messages
}

fn bench_commit_parsing(c: &mut Criterion) {
    for size in [100, 1_000, 10_000] {
        let messages = generate_commit_messages(size);
        c.bench_function(&format!("commit_parsing/{size}"), |b| {
            b.iter(|| {
                for msg in &messages {
                    black_box(determine_bump(msg));
                }
            });
        });
    }
}

criterion_group!(benches, bench_commit_parsing);
criterion_main!(benches);
```

- [ ] **Step 2: Verify it compiles and runs**

```bash
cargo bench --bench ferrflow_benchmarks
```

Expected: 3 benchmarks run (commit_parsing/100, commit_parsing/1000, commit_parsing/10000), each showing timing results.

- [ ] **Step 3: Commit**

```bash
git add benches/ferrflow_benchmarks.rs
git commit -m "feat(bench): add criterion commit_parsing micro-benchmarks"
```

---

### Task 3: Add changelog benchmarks

**Files:**
- Modify: `benches/ferrflow_benchmarks.rs`

- [ ] **Step 1: Add changelog benchmark group**

Add these imports at the top of `benches/ferrflow_benchmarks.rs`:

```rust
use ferrflow::changelog::{build_section, update_changelog};
use ferrflow::conventional_commits::BumpType;
use ferrflow::git::GitLog;
use std::io::Write;
use tempfile::NamedTempFile;
```

Add the benchmark function after `bench_commit_parsing`:

```rust
fn generate_commits(count: usize) -> Vec<GitLog> {
    let types = ["feat", "fix", "refactor", "perf", "chore", "docs"];
    let scopes = ["api", "auth", "db", "cache", "config"];
    let mut commits = Vec::with_capacity(count);
    for i in 0..count {
        let t = types[i % types.len()];
        let s = scopes[i % scopes.len()];
        let breaking = if i % 20 == 0 { "!" } else { "" };
        commits.push(GitLog {
            hash: format!("{i:08x}"),
            message: format!("{t}({s}){breaking}: change number {i}"),
        });
    }
    commits
}

fn bench_changelog(c: &mut Criterion) {
    for size in [50, 500] {
        let commits = generate_commits(size);

        c.bench_function(&format!("changelog/build_{size}"), |b| {
            b.iter(|| {
                black_box(build_section("1.0.0", &commits));
            });
        });

        c.bench_function(&format!("changelog/update_{size}"), |b| {
            b.iter(|| {
                let mut f = NamedTempFile::new().unwrap();
                f.write_all(b"# Changelog\n\n## v0.9.0\n\n- old entry\n").unwrap();
                let path = f.path().to_path_buf();
                black_box(
                    update_changelog(&path, "myapp", "1.0.0", &commits, BumpType::Minor, false)
                        .unwrap(),
                );
            });
        });
    }
}
```

Update the `criterion_group!` macro to include the new group:

```rust
criterion_group!(benches, bench_commit_parsing, bench_changelog);
```

- [ ] **Step 2: Verify it compiles and runs**

```bash
cargo bench --bench ferrflow_benchmarks
```

Expected: 7 benchmarks total (3 commit_parsing + 4 changelog).

- [ ] **Step 3: Commit**

```bash
git add benches/ferrflow_benchmarks.rs
git commit -m "feat(bench): add criterion changelog micro-benchmarks"
```

---

### Task 4: Add version_files benchmarks

**Files:**
- Modify: `benches/ferrflow_benchmarks.rs`

- [ ] **Step 1: Add version_files benchmark group**

Add this import at the top:

```rust
use ferrflow::formats::get_handler;
use ferrflow::config::FileFormat;
```

Add the benchmark function:

```rust
fn bench_version_files(c: &mut Criterion) {
    let cases: Vec<(&str, FileFormat, &str)> = vec![
        (
            "toml",
            FileFormat::Toml,
            "[package]\nname = \"foo\"\nversion = \"1.2.3\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
        ),
        (
            "json",
            FileFormat::Json,
            r#"{"name":"foo","version":"1.2.3","description":"a package","main":"index.js"}"#,
        ),
        (
            "xml",
            FileFormat::Xml,
            "<project>\n  <modelVersion>4.0.0</modelVersion>\n  <groupId>com.example</groupId>\n  <artifactId>foo</artifactId>\n  <version>1.2.3</version>\n</project>\n",
        ),
        (
            "gradle",
            FileFormat::Gradle,
            "plugins { id 'java' }\nversion = \"1.2.3\"\ngroup = 'com.example'\n",
        ),
    ];

    for (name, format, content) in &cases {
        let handler = get_handler(format);

        c.bench_function(&format!("version_files/{name}_read"), |b| {
            let mut f = NamedTempFile::new().unwrap();
            f.write_all(content.as_bytes()).unwrap();
            let path = f.path().to_path_buf();
            b.iter(|| {
                black_box(handler.read_version(&path).unwrap());
            });
        });

        c.bench_function(&format!("version_files/{name}_write"), |b| {
            let mut f = NamedTempFile::new().unwrap();
            f.write_all(content.as_bytes()).unwrap();
            let path = f.path().to_path_buf();
            b.iter(|| {
                black_box(handler.write_version(&path, "2.0.0").unwrap());
            });
        });
    }
}
```

Update the `criterion_group!`:

```rust
criterion_group!(benches, bench_commit_parsing, bench_changelog, bench_version_files);
```

- [ ] **Step 2: Verify it compiles and runs**

```bash
cargo bench --bench ferrflow_benchmarks
```

Expected: 15 benchmarks total (3 + 4 + 8).

- [ ] **Step 3: Commit**

```bash
git add benches/ferrflow_benchmarks.rs
git commit -m "feat(bench): add criterion version_files micro-benchmarks"
```

---

### Task 5: Add config_loading benchmarks

**Files:**
- Modify: `benches/ferrflow_benchmarks.rs`

- [ ] **Step 1: Add config_loading benchmark group**

Add this import at the top:

```rust
use ferrflow::config::Config;
use tempfile::TempDir;
```

Add the benchmark function:

```rust
fn generate_config_json(num_packages: usize) -> String {
    let mut packages = Vec::new();
    for i in 1..=num_packages {
        packages.push(format!(
            r#"    {{
      "name": "pkg-{i:03}",
      "path": "packages/pkg-{i:03}",
      "changelog": "packages/pkg-{i:03}/CHANGELOG.md",
      "versioned_files": [
        {{ "path": "packages/pkg-{i:03}/package.json", "format": "json" }}
      ]
    }}"#
        ));
    }
    format!("{{\n  \"package\": [\n{}\n  ]\n}}", packages.join(",\n"))
}

fn bench_config_loading(c: &mut Criterion) {
    for (label, num_pkgs) in [("single", 1), ("mono_10", 10), ("mono_50", 50)] {
        c.bench_function(&format!("config_loading/{label}"), |b| {
            let dir = TempDir::new().unwrap();
            let config_path = dir.path().join(".ferrflow");
            std::fs::write(&config_path, generate_config_json(num_pkgs)).unwrap();
            // Config::load needs a git repo root, but we only need it to find the config file.
            // Initialize a bare git repo in the temp dir so Config::load works.
            std::process::Command::new("git")
                .args(["init", "-q"])
                .current_dir(dir.path())
                .output()
                .unwrap();
            b.iter(|| {
                black_box(Config::load(dir.path(), None).unwrap());
            });
        });
    }
}
```

Update the `criterion_group!`:

```rust
criterion_group!(
    benches,
    bench_commit_parsing,
    bench_changelog,
    bench_version_files,
    bench_config_loading
);
```

- [ ] **Step 2: Verify all benchmarks run**

```bash
cargo bench --bench ferrflow_benchmarks
```

Expected: 18 benchmarks total (3 + 4 + 8 + 3).

- [ ] **Step 3: Commit**

```bash
git add benches/ferrflow_benchmarks.rs
git commit -m "feat(bench): add criterion config_loading micro-benchmarks"
```

---

### Task 6: Add micro-bench CI job

**Files:**
- Modify: `.github/workflows/ci.yml`

- [ ] **Step 1: Add the micro-bench job**

Add this job after the `coverage` job and before the `benchmark` job in `.github/workflows/ci.yml`:

```yaml
  micro-bench:
    name: Micro Benchmarks
    runs-on: ubuntu-latest
    permissions:
      pull-requests: write
    steps:
      - uses: actions/checkout@v6
      - uses: dtolnay/rust-toolchain@nightly
      - uses: Swatinem/rust-cache@v2
      - name: Run benchmarks
        run: cargo bench --bench ferrflow_benchmarks -- --output-format bencher 2>/dev/null | tee output.txt
      - name: Download baseline
        if: github.event_name == 'pull_request'
        uses: actions/download-artifact@v4
        with:
          name: criterion-baseline
          path: baseline/
        continue-on-error: true
      - name: Compare and comment
        if: github.event_name == 'pull_request'
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: cargo
          output-file-path: output.txt
          external-data-json-path: baseline/benchmark-data.json
          comment-on-alert: true
          alert-threshold: '120%'
          fail-on-alert: false
          github-token: ${{ secrets.GITHUB_TOKEN }}
          comment-always: true
      - name: Prepare baseline data
        if: github.event_name == 'push' && github.ref == 'refs/heads/main'
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: cargo
          output-file-path: output.txt
          external-data-json-path: baseline/benchmark-data.json
          save-data-file: true
      - name: Upload baseline
        if: github.event_name == 'push' && github.ref == 'refs/heads/main'
        uses: actions/upload-artifact@v4
        with:
          name: criterion-baseline
          path: baseline/benchmark-data.json
          retention-days: 90
          overwrite: true
```

- [ ] **Step 2: Verify YAML is valid**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))" && echo "Valid YAML"
```

Expected: "Valid YAML"

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add criterion micro-bench job with PR comments"
```

---

### Task 7: Push and create PR

- [ ] **Step 1: Create the issue**

```bash
gh issue create --repo FerrFlow-Org/FerrFlow \
  --title "Add Criterion micro-benchmarks with PR comments" \
  --body "Add Criterion-based micro-benchmarks for internal subsystems (commit parsing, changelog generation, version file handling, config loading). Run on every PR with automated comparison against main baseline, posted as a PR comment via github-action-benchmark. Warning only, no CI failure on regression."
```

- [ ] **Step 2: Push and create PR**

```bash
git push -u origin feat/criterion-microbenchmarks
gh pr create \
  --title "feat(bench): add Criterion micro-benchmarks with PR comments" \
  --body "$(cat <<'EOF'
## Summary
- Add lib target to expose internal modules to benchmarks
- 18 Criterion micro-benchmarks across 4 groups: commit parsing, changelog, version files, config loading
- New micro-bench CI job runs on every PR and push to main
- github-action-benchmark compares against baseline and posts a comparison table in PRs
- Warning only (alert threshold 120%, no CI failure)
- Baseline stored as GitHub Actions artifact (90-day retention)

Closes #<issue_number>

## Test plan
- [ ] cargo bench runs all 18 benchmarks successfully
- [ ] CI job posts comparison comment on PR
- [ ] First run on main uploads baseline artifact
- [ ] Subsequent PR runs compare against baseline
EOF
)"
```
