# syntax=docker/dockerfile:1.7

# ─── Stage 1: builder ─────────────────────────────────────────────────────
FROM rust:1.95-bookworm AS builder

WORKDIR /build

# Cache dependency layer
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo 'fn main(){}' > src/main.rs && \
    echo '' > src/lib.rs && \
    cargo build --release --bin agk && \
    rm -rf src

# Build sources
COPY . .
ARG FEATURES="tui"
RUN cargo build --release --bin agk --no-default-features --features "${FEATURES}"

# ─── Stage 2: slim runtime ────────────────────────────────────────────────
FROM debian:bookworm-slim AS slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates tini && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/agk /usr/local/bin/agk

# tini = PID 1, proper signal forwarding
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/agk"]
CMD ["--help"]

# ─── Stage 3: full CI image (sources + build cache, no compiled binary) ──
FROM rust:1.95-bookworm AS full

WORKDIR /build
RUN apt-get update && \
    apt-get install -y --no-install-recommends git ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY . .
# Pre-warm the build cache for the full feature set
RUN cargo build --release --bin agk \
    --features "tui,llm-ollama,llm-lmstudio,llm-anthropic,llm-openai,profile-create,claude-cli-probe,pack"

ENTRYPOINT ["cargo"]
CMD ["test", "--lib", "--all-features"]
