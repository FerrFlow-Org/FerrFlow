# Changelog

All notable changes to `ferrflow` will be documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [0.6.0] - 2026-03-28

### Features

- feat: add version and tag query commands for CI scripting (#74)
- feat: add configurable tag prefix (#72)
- feat(versioning): support per-package versioning strategies (#70)
- feat(ci): add benchmark suite comparing against competitors (#67)
- feat(config): add explicit config path and ambiguity guard (#66)
- Feat/json json5 config (#63)
- feat: add telemetry module with fire-and-forget usage stats (#61)
- Feat/json json5 config (#59)
- Feat/json json5 config (#58)
- feat: support ferrflow.json and ferrflow.json5 config formats (#57)
- Feat/status command (#41)
- feat: write release summary to GITHUB_STEP_SUMMARY (#40)
- feat(status): add status command (#34)
- Feat/GitHub action (#24)
- feat: detect default branch from git remote instead of hardcoding main (#19)
- feat: add GitHub Action for public use (#15)
- feat: create GitHub Release via API after push (#12)
- feat: implement standalone changelog command (#11)
- feat: fallback to FerrFlow identity when git user not configured
- feat: auto-commit and push after release bump
- feat: initial FerrFlow implementation

### Bug Fixes

- fix(bench): configure git identity in fixture generator (#68)
- fix: handle orphaned release tags (#56)
- fix(deps): update rust crate toml_edit to 0.25 (#52)
- fix(deps): update rust crate quick-xml to 0.39 (#50)
- fix: vendor libgit2 in Dockerfile to fix Alpine musl build (#43)
- fix: push tags individually instead of glob refspec

## [0.4.0] - 2026-03-26

### Features

- feat: add GitHub Action for public use
- feat: detect default branch from git remote instead of hardcoding main
- feat: implement standalone changelog command
- feat: create GitHub Release via API after push
- feat: add status command
- feat: write release summary to GITHUB_STEP_SUMMARY

### Bug Fixes

- fix: vendor libgit2 and openssl to support musl and macOS cross-compilation

### Chores

- ci: release workflow now triggered by published GitHub release event

## [0.3.0] - 2026-03-24

### Features

- feat: fallback to FerrFlow identity when git user not configured

## [0.2.0] - 2026-03-24

### Features

- feat: auto-commit and push after release bump
- feat: initial FerrFlow implementation

### Bug Fixes

- fix: push tags individually instead of glob refspec
