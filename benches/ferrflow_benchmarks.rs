use std::io::Write;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use ferrflow::changelog::{build_section, update_changelog};
use ferrflow::config::{Config, FileFormat};
use ferrflow::conventional_commits::{BumpType, determine_bump};
use ferrflow::formats::get_handler;
use ferrflow::git::GitLog;
use tempfile::{NamedTempFile, TempDir};

fn generate_commit_messages(count: usize) -> Vec<String> {
    let types = [
        "feat", "fix", "refactor", "perf", "chore", "docs", "ci", "test",
    ];
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

fn generate_commits(count: usize) -> Vec<GitLog> {
    let types = [
        "feat", "fix", "refactor", "perf", "chore", "docs", "ci", "test",
    ];
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
                f.write_all(b"# Changelog\n\n## v0.9.0\n\n- old entry\n")
                    .unwrap();
                let path = f.path().to_path_buf();
                black_box(
                    update_changelog(&path, "myapp", "1.0.0", &commits, BumpType::Minor, false)
                        .unwrap(),
                );
            });
        });
    }
}

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

criterion_group!(
    benches,
    bench_commit_parsing,
    bench_changelog,
    bench_version_files,
    bench_config_loading
);
criterion_main!(benches);
