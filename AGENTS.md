# OBS HomeRun - Agent & Contributor Guidelines

This document provides essential instructions and guidelines for AI coding agents and contributors working in this repository.

---

## 1. Release Versioning and Tagging Rules

> [!IMPORTANT]
> **Always bump `version` in `Cargo.toml` and create a matching Git tag when modifying application code.**

Whenever changes are made to application code (`src/`, `Cargo.toml`, `Containerfile`, or runtime configs):

1. **Bump Version**: Update `version` in `Cargo.toml` following Semantic Versioning (`MAJOR.MINOR.PATCH`).
2. **Update Lockfile**: Run tests or `cargo check` to synchronize `Cargo.lock`.
3. **Commit Changes**: Use Conventional Commits (e.g., `fix(stream): ...`, `feat(engine): ...`).
4. **Create Annotated Tag**: Create a git tag matching the version format `vX.Y.Z`:
   ```bash
   git tag -a vX.Y.Z -m "Release vX.Y.Z: Summary of changes"
   ```
5. **Push Branch and Tags**:
   ```bash
   git push origin main --tags
   ```

### Why Tagging is Required
Our GitHub Actions workflow ([`.github/workflows/docker-publish.yml`](.github/workflows/docker-publish.yml)) publishes multi-arch container images to GitHub Container Registry (GHCR) using `docker/metadata-action`:
* **Tagged pushes (`v*.*.*`)**: Automatically build and publish tagged container releases (`:vX.Y.Z`, `:X.Y`, `:X`).
* **Branch pushes (`main`)**: Only publish `:latest` and `:sha-<commit>`.
Without a Git tag, users and deployments pinning release versions will never receive updates.

---

## 2. Testing and Validation Runbook

Before committing any changes to `src/` or `tests/`, always run the test suite and linter.

### In Container (Standard Environment)
If local Rust toolchains are not in host `PATH`, use the container runtime:

* **Code Formatting**:
  ```bash
  # Check formatting
  podman run --rm -v "$(pwd)":/src:Z -w /src rust:alpine sh -c "rustup component add rustfmt 2>/dev/null; cargo fmt --check"

  # Autoformat files
  podman run --rm -v "$(pwd)":/src:Z -w /src rust:alpine sh -c "rustup component add rustfmt 2>/dev/null; cargo fmt"
  ```
* **Pedantic Clippy Linter Check**:
  ```bash
  podman run --rm -v "$(pwd)":/src:Z -w /src rust:alpine sh -c "rustup component add clippy 2>/dev/null; cargo clippy --all-targets --all-features -- -D warnings"
  ```
  *(Note: Strict `pedantic` and `nursery` lints plus `forbid(unsafe_code)` are configured in `Cargo.toml` under `[lints]`, so `-D warnings` automatically enforces them).*
* **Unit & Protocol Tests**:
  ```bash
  podman run --rm -v "$(pwd)":/src:Z -w /src rust:alpine cargo test
  ```

---

## 3. Streaming and Process Resilience Guidelines

* **MediaMTX Supervision**:
  Embedded MediaMTX is managed by a background supervisor task in `src/main.rs`. MediaMTX binary discovery searches `MEDIAMTX_BIN`, alongside the executable, relative paths, the system `PATH` (`std::env::split_paths`), and standard system locations.
* **FFmpeg Demuxer Timeouts**:
  Always maintain `-timeout 5000000` (5-second socket timeout in microseconds) before `-i` in `src/stream.rs` to avoid blocking indefinitely on stalled or half-open RTSP connections.
* **MPEG-TS Stream Discontinuity**:
  Do not remove `-mpegts_flags +initial_discontinuity+resend_headers` in `src/stream.rs`. This enables smart TV decoders to absorb stream drops and broadcaster restarts without triggering player errors or lost signal screens.
