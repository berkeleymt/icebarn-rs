# syntax=docker/dockerfile:1

###############################################################################
# Builder stage: compile the Leptos (SSR) server binary + hashed static site.
#
# cargo-leptos drives the whole build: it compiles the `ssr` server binary,
# compiles the `hydrate` lib to wasm32, runs Tailwind (auto-downloads the
# standalone binary), and emits everything under target/site.
###############################################################################
FROM rustlang/rust:nightly-bookworm AS builder

# wasm target for the hydrate (client) bundle.
RUN rustup target add wasm32-unknown-unknown

# Install cargo-leptos. cargo-binstall fetches a prebuilt binary when available,
# which is far faster than `cargo install --locked` from source.
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
        https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz \
        | tar -xzf - -C /usr/local/cargo/bin \
    && cargo binstall -y cargo-leptos

WORKDIR /app

# Copy the full source. (.dockerignore keeps target/, .git/, etc. out.)
COPY . .

# Build release artifacts: server binary + hashed static site under target/site.
RUN cargo leptos build --release -vv

###############################################################################
# Runtime stage: minimal image with just the server binary + static site.
###############################################################################
FROM debian:bookworm-slim AS runtime

# ca-certificates is required for outbound HTTPS (ContestDojo OIDC/API in the
# OAuth feature). curl is included for the container healthcheck.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# Run as a non-root user.
RUN useradd --create-home --uid 10001 appuser
WORKDIR /app

# The compiled server binary.
COPY --from=builder /app/target/release/icebarn-rs /app/icebarn-rs
# The hashed static site (JS/WASM/CSS + public assets). Served from LEPTOS_SITE_ROOT.
COPY --from=builder /app/target/site /app/site

# Leptos runtime configuration. site-addr MUST bind 0.0.0.0 to be reachable
# from outside the pod; site-root points at the copied static dir.
ENV LEPTOS_OUTPUT_NAME="icebarn-rs" \
    LEPTOS_SITE_ROOT="site" \
    LEPTOS_SITE_PKG_DIR="pkg" \
    LEPTOS_SITE_ADDR="0.0.0.0:3000" \
    LEPTOS_RELOAD_PORT="3001" \
    RUST_LOG="info"

USER appuser
EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -fsS http://localhost:3000/ || exit 1

CMD ["/app/icebarn-rs"]
