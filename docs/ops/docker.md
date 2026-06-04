# Docker — agk runtime & CI images

> Operator-facing guide to the two images produced by `docker/Dockerfile`: the
> production **slim** runtime, and the **full** CI image that runs the test
> matrix.

## Overview

`docker/Dockerfile` is a multi-stage build that produces two named targets.

| Target | Base                    | Purpose                                                                          | When to use                                                                 |
| ------ | ----------------------- | -------------------------------------------------------------------------------- | --------------------------------------------------------------------------- |
| `slim` | `debian:bookworm-slim`  | Minimal runtime with the `agk` binary; TUI-capable; no source, no Rust toolchain | Production runs, CI jobs that only invoke `agk`, local dev containers       |
| `full` | `rust:1.95-bookworm`    | Full Rust toolchain + sources + warm build cache for the entire feature set      | Reproducing the CI test matrix locally, debugging build issues, hermetic QA |

### Why two images

The slim image strips everything except the binary, `ca-certificates`, and
`tini`. It exists so production containers don't pay for a Rust toolchain they
will never use. The full image exists so CI can build a hermetic environment
that matches the test matrix without depending on host state.

The two targets are independent: building `slim` does not require building
`full`, and vice versa.

## Build

The Dockerfile lives at `docker/Dockerfile` (relative to the repo root). The
`.dockerignore` next to it excludes `target/`, `.git/`, `docs/`, and editor
metadata to keep the build context small.

### Slim runtime

```bash
# Build with default features (TUI only)
docker build -f docker/Dockerfile --target slim -t agk:slim .

# Build with extra features (e.g. pack, profile-create, llm-ollama)
docker build \
  --build-arg FEATURES="tui,pack,profile-create,llm-ollama" \
  -f docker/Dockerfile --target slim \
  -t agk:slim-llm .
```

The `FEATURES` build arg is forwarded to `cargo build
--no-default-features --features "${FEATURES}"`. Whatever you pass becomes the
binary's compile-time feature set. If you want only the TUI (the smallest
binary), pass `--build-arg FEATURES=tui`.

### Full CI image

```bash
docker build -f docker/Dockerfile --target full -t agk:full .
```

The full target always compiles with the full feature set baked into the cache
(`tui,llm-ollama,llm-lmstudio,llm-anthropic,llm-openai,profile-create,claude-cli-probe,pack`),
so subsequent `cargo test` invocations inside the container are fast.

## Run slim

The slim image's `ENTRYPOINT` is `tini -- /usr/local/bin/agk`, which means
`agk` runs as PID 1 with proper signal forwarding (SIGTERM, SIGINT, SIGQUIT
reach the binary, not a zombie init).

### Interactive TUI

```bash
# Mount your workspace so agk can read .agk/config.toml
docker run --rm -it \
  -v "$PWD":/workspace \
  -w /workspace \
  agk:slim
```

Allocate a pseudo-TTY with `-it`. The `-w /workspace` flag sets the working
directory inside the container, which is what `agk` uses to locate
`.agk/config.toml`.

### Headless commands

```bash
# Sync all configured assets
docker run --rm \
  -v "$PWD":/workspace \
  -w /workspace \
  agk:slim \
  agk sync --json

# Install a specific skill
docker run --rm \
  -v "$HOME/.config/agk":/config \
  -v "$PWD":/workspace \
  -w /workspace \
  -e XDG_CONFIG_HOME=/config \
  agk:slim \
  agk install clawhub/web-browser
```

Note `-e XDG_CONFIG_HOME=/config`: the `agk` binary reads its global config
from `$XDG_CONFIG_HOME/agk/` (or `~/.config/agk/`). Mounting your host
`~/.config/agk` lets the container reuse the same vault registrations,
providers, and LLM provider configs you have on your machine.

### One-off help / version

```bash
docker run --rm agk:slim --version
docker run --rm agk:slim --help
```

The default `CMD` is `--help`, so a bare `docker run agk:slim` prints usage.

## Run full

The full image is meant for running the test matrix. It does **not** ship a
pre-built `agk` binary — the entrypoint is `cargo`, not `agk`.

```bash
# Default: cargo test --lib --all-features
docker run --rm agk:full

# Custom test invocation
docker run --rm agk:full cargo test --workspace --all-targets --all-features

# Run a single test by name
docker run --rm agk:full cargo test profile_start_dry_run_matches_contract_fixture -- --nocapture

# Architecture tests (the --ignored ones)
docker run --rm agk:full cargo test --test architecture -- --ignored
```

Because the build cache is baked into the image, the first `cargo test` is
slow (it re-runs the dependency compilation the build step produced); the
second and subsequent runs are fast.

## Image sizes

Approximate sizes for the default build on `linux/amd64`:

| Target | Base                | Uncompressed | What dominates the size                          |
| ------ | ------------------- | ------------ | ------------------------------------------------ |
| `slim` | `debian:bookworm-slim` | ~50 MB    | debian-slim base + tini + ca-certificates        |
| `full` | `rust:1.95-bookworm`   | ~2-3 GB   | Rust toolchain + target/ build cache             |

Compressed sizes are roughly half of the numbers above when pushed to a
registry. The slim image is small enough to embed in sidecar containers and
multi-stage release pipelines.

## Security

Both stages apply the following defaults:

- **Tini as PID 1** (slim stage only). `tini` reaps zombie processes and
  forwards signals to the child. Without it, a Rust binary running as PID 1
  in a container silently ignores `SIGTERM`, and `docker stop` falls back to
  `SIGKILL` after a 10-second grace period.
- **Non-root user** — the slim stage currently runs as root. If you need
  unprivileged execution, add `USER agk` (or any UID) before the
  `ENTRYPOINT`. The full stage is intended to run as root inside CI; do not
  expose it externally.
- **`ca-certificates` only** (slim). The slim image installs the minimum set
  of apt packages needed at runtime — nothing else. No `wget`, `curl`, or
  `git`.
- **Distroless alternative.** For a smaller attack surface, swap the slim
  stage's base for `gcr.io/distroless/cc-debian12` and copy `tini` and `agk`
  in. This removes `/bin/sh`, all package metadata, and the ability to run
  arbitrary commands in the container.
- **No secrets baked in.** The `FEATURES` build arg is the only build-time
  secret; runtime config (API keys, vault tokens) is read from environment
  variables and mounted config files, never `ARG`-injected.

## Multi-arch

The Dockerfile is architecture-agnostic, so it builds natively on any host
that Docker supports. To produce a manifest list for both `linux/amd64` and
`linux/arm64`:

```bash
docker buildx create --name agk-builder --use
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  --target slim \
  -t ghcr.io/dkthezero/agk:slim \
  --push \
  -f docker/Dockerfile .
```

Replace `--target slim` with `--target full` to publish the CI image. The
buildx cache will be reused across architectures if you mount a cache backend
(see the Docker docs for `--cache-to=type=registry,ref=...`).

The `FROM rust:1.95-bookworm` line in the full stage pulls the multi-arch
manifest of the official `rust` image, which is published for both `amd64`
and `arm64`. The slim stage's `debian:bookworm-slim` is also multi-arch.

## Troubleshooting

### "Workspace config not found" / empty state

agk looks for `.agk/config.toml` in the current working directory. If you
forgot `-w /workspace` or the `-v "$PWD":/workspace` mount, the container
sees an empty `/` directory and agk starts with no installed assets.

```bash
# Verify the mount
docker run --rm -v "$PWD":/workspace -w /workspace agk:slim ls -la .agk
```

### TUI does not render / exits immediately

Two common causes:

1. **No TTY.** Make sure you pass `-it` (or `--tty --interactive`).
2. **`TERM` not set.** Some hosts strip `TERM` from the container
   environment. Pass `-e TERM=xterm-256color` (or whatever your local
   `$TERM` is). The TUI uses ratatui, which renders nothing if `TERM` is
   unknown.

### `docker stop` takes 10 seconds

This is the symptom of running a Rust binary as PID 1 without `tini`. The
slim image already wraps the binary in `tini` — if you replaced the
`ENTRYPOINT`, put `tini` back. The full image uses `cargo` as its
entrypoint, which is fine to run as PID 1 because it traps SIGTERM.

### `~/.config/agk` is empty inside the container

The default `XDG_CONFIG_HOME` inside the container is `/root` (because the
container runs as root). To use your host config, mount it and override
the env var:

```bash
docker run --rm \
  -v "$HOME/.config/agk":/root/.config/agk \
  -v "$PWD":/workspace \
  -w /workspace \
  agk:slim \
  agk sync
```

### `cargo build` fails with "feature ... not found" inside the full image

The full image's cached feature set is fixed at build time. If you need a
new feature combination, rebuild the full image with an updated
`cargo build` line in the Dockerfile (or just run the build inside the
container with a one-off feature set).
