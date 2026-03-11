# Session Context

## User Prompts

### Prompt 1

Implement the following plan:

# Plan: Migrate to Date-Based Release Versioning (YYYYMMDD.NN)

## Context

Migrate moltis from semver (`v0.10.18`) to date-based versioning (`20260311.01`) matching the arbor project pattern. The user-facing version becomes `YYYYMMDD.NN` (date + daily sequence number). Cargo.toml stays at a static `0.1.0` since Cargo enforces semver, and the real version is injected at build time via `MOLTIS_VERSION` env var.

## 1. Runtime Version Resolution

**Files:**
- `cra...

### Prompt 2

commit, push, create a PR

