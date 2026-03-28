# Criterion Micro-Benchmarks

Add Criterion-based micro-benchmarks that run on every PR, with automated comparison against main and a comment posted in the PR showing results.

## Goals

1. **Early regression detection** -- catch performance regressions before merge, on every PR.
2. **Sub-system visibility** -- measure individual components (commit parsing, changelog, version files, config loading) rather than end-to-end CLI invocations.
3. **PR feedback** -- post a comparison table in the PR so reviewers can see the impact.

## Relationship to Existing Benchmarks

| | Hyperfine (end-to-end) | Criterion (micro) |
|---|---|---|
| When | Push to main only | Every PR + push to main |
| Speed | ~5-10 min | ~30s |
| What it measures | Full CLI commands against synthetic repos | Internal functions with synthetic data |
| In release notes | Yes (Markdown tables) | No |
| Regression behavior | Fail the job | Warning only (comment in PR) |
| Baseline storage | Committed JSON file | GitHub Actions artifact (90 days) |

These are complementary. Criterion catches micro-regressions fast on every PR. Hyperfine validates real-world performance on main.

## Benchmark Groups

Four groups, all in a single file `benches/ferrflow_benchmarks.rs`:

### commit_parsing

Measures `determine_bump()` called on batches of synthetic conventional commit messages.

| Benchmark | Input |
|-----------|-------|
| `commit_parsing/100` | 100 commit messages |
| `commit_parsing/1000` | 1,000 commit messages |
| `commit_parsing/10000` | 10,000 commit messages |

Commit messages are generated in-memory: random type/scope/description, ~5% breaking changes. Same distribution as the fixture generator.

### changelog

Measures `build_section()` and `update_changelog()` with varying commit counts.

| Benchmark | Input |
|-----------|-------|
| `changelog/build_50` | `build_section()` with 50 commits |
| `changelog/build_500` | `build_section()` with 500 commits |
| `changelog/update_50` | `update_changelog()` with 50 commits (writes to tempfile) |
| `changelog/update_500` | `update_changelog()` with 500 commits (writes to tempfile) |

### version_files

Measures `read_version()` and `write_version()` for each supported format. Each benchmark creates a tempfile with realistic content, then reads/writes the version field.

| Benchmark | Input |
|-----------|-------|
| `version_files/toml_read` | Cargo.toml with typical fields |
| `version_files/toml_write` | Same file, write new version |
| `version_files/json_read` | package.json with typical fields |
| `version_files/json_write` | Same file, write new version |
| `version_files/xml_read` | pom.xml with typical structure |
| `version_files/xml_write` | Same file, write new version |
| `version_files/gradle_read` | build.gradle with version field |
| `version_files/gradle_write` | Same file, write new version |

### config_loading

Measures `Config::load()` with config files of different sizes, written to tempfiles.

| Benchmark | Input |
|-----------|-------|
| `config_loading/single` | .ferrflow with 1 package |
| `config_loading/mono_10` | .ferrflow with 10 packages |
| `config_loading/mono_50` | .ferrflow with 50 packages |

## File Structure

```
benches/
  ferrflow_benchmarks.rs      # all 4 groups in one file

Cargo.toml                    # criterion dev-dependency + [[bench]] section

.github/workflows/ci.yml      # new micro-bench job
```

One file because each group is ~30 lines and they share imports. Criterion organizes them into named groups internally.

## Dependencies

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "ferrflow_benchmarks"
harness = false
```

## CI Integration

A new job `micro-bench` in `.github/workflows/ci.yml`.

### On pull requests

1. `cargo bench --bench ferrflow_benchmarks -- --output-format bencher`
2. Download baseline artifact from the last main run (if it exists)
3. `github-action-benchmark` compares results against baseline
4. Posts a comment in the PR with the full comparison table
5. Job stays green regardless of regressions (warning only)

### On push to main

1. `cargo bench --bench ferrflow_benchmarks -- --output-format bencher`
2. Upload results as artifact `criterion-baseline` (90-day retention)
3. No comment posted

### CI job definition

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
      run: cargo bench --bench ferrflow_benchmarks -- --output-format bencher | tee output.txt
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
    - name: Upload baseline
      if: github.event_name == 'push' && github.ref == 'refs/heads/main'
      uses: actions/upload-artifact@v4
      with:
        name: criterion-baseline
        path: output.txt
        retention-days: 90
    - name: Store for action
      if: github.event_name == 'push' && github.ref == 'refs/heads/main'
      uses: benchmark-action/github-action-benchmark@v1
      with:
        tool: cargo
        output-file-path: output.txt
        external-data-json-path: baseline/benchmark-data.json
        save-data-file: true
    - name: Upload baseline data
      if: github.event_name == 'push' && github.ref == 'refs/heads/main'
      uses: actions/upload-artifact@v4
      with:
        name: criterion-baseline
        path: baseline/benchmark-data.json
        retention-days: 90
        overwrite: true
```

### PR Comment Format

`github-action-benchmark` posts a comment like:

```
## Benchmark Results

| Test | Base | Current | Ratio |
|------|------|---------|-------|
| commit_parsing/100 | 12.3 us (2.1 us) | 12.1 us (1.8 us) | 0.98 |
| commit_parsing/1000 | 125.4 us (5.2 us) | 124.8 us (4.9 us) | 1.00 |
| changelog/build_50 | 45.2 us (3.1 us) | 44.8 us (2.8 us) | 0.99 |
| version_files/toml_read | 8.1 us (0.5 us) | 8.0 us (0.4 us) | 0.99 |
| config_loading/single | 22.1 us (1.5 us) | 22.0 us (1.4 us) | 1.00 |

Ratio > 1 = slower, < 1 = faster
```

Alert threshold is 120% (ratio 1.2). Values between parentheses are stddev. The comment is updated on each push to the PR branch (not duplicated).

## No Emojis

The `github-action-benchmark` action does not add emojis by default in table format. If any appear in the alert message, they will be stripped or the template will be overridden.
