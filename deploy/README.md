# Building & running the icebarn-rs container image

`icebarn-rs` is a Leptos (SSR) + Axum app. This directory documents the
container image defined by the repo-root [`Dockerfile`](../Dockerfile): how to
build, push, and run it, plus the runtime environment variables it expects.

> Kubernetes manifests are **not** kept in this repo — they live in the
> deployment/infra setup. This doc only covers the image itself and the env
> contract it must be given at runtime.

## Build & push

```bash
# From the repo root
docker build -t <REGISTRY>/icebarn-rs:<TAG> .
docker push   <REGISTRY>/icebarn-rs:<TAG>
```

The multi-stage `Dockerfile`:

1. **builder** (`rustlang/rust:nightly-bookworm`): adds the
   `wasm32-unknown-unknown` target, installs `cargo-leptos` (via `cargo-binstall`),
   and runs `cargo leptos build --release`. `cargo-leptos` compiles the SSR
   server binary, compiles the hydrate lib to WASM, and runs Tailwind
   (auto-downloading the standalone binary), emitting the hashed static site to
   `target/site`.
2. **runtime** (`debian:bookworm-slim`): a small image (~110 MB) containing only
   the server binary (`/app/icebarn-rs`), the static site (`/app/site`), and
   `ca-certificates` (for outbound HTTPS to ContestDojo). Runs as a non-root
   user and exposes port `3000`.

### Why nightly + wasm

`cargo-leptos` builds the client bundle as WebAssembly and the toolchain is
nightly (per the upstream Leptos/Axum template and the project README). The
builder stage honors this. There is no `rust-toolchain.toml` in the repo, so the
builder pins nightly via the base image.

## Run (smoke test)

```bash
# Throwaway Postgres
docker run -d --name icebarn-pg \
  -e POSTGRES_USER=icebarn -e POSTGRES_PASSWORD=icebarn -e POSTGRES_DB=icebarn \
  -p 5432:5432 postgres:16

# App (host networking so it can reach the Postgres above)
docker run --rm --network host \
  -e POSTGRES_URI='postgres://icebarn:icebarn@localhost:5432/icebarn' \
  <REGISTRY>/icebarn-rs:<TAG>

curl -i http://localhost:3000/   # -> HTTP/1.1 200 OK
```

The server logs `listening on http://0.0.0.0:3000` once up.

## Runtime environment variables

The image sets the `LEPTOS_*` runtime variables itself (see the `Dockerfile`);
notably `LEPTOS_SITE_ADDR=[::]:3000` (IPv6 any, **dual-stack** on Linux so it
also accepts IPv4-mapped connections — the upstream default `127.0.0.1:3000` is
**not** reachable from outside the container/pod) and `LEPTOS_SITE_ROOT=site`.

### Required

| Variable | Notes |
| --- | --- |
| `POSTGRES_URI` | Postgres connection string. App auto-creates the `rooms` table on boot. **App panics if unset.** |

### Required for OAuth login (PR #4 — "Sign in with ContestDojo")

If these are unset, the auth routes render a "not configured" page and the lobby
only offers single-player; the app still runs.

| Variable | Sensitive | Notes |
| --- | --- | --- |
| `OIDC_CLIENT_ID` | no | e.g. `bmmt_puzzle` |
| `OIDC_CLIENT_SECRET` | **yes** | OIDC client secret |
| `OIDC_REDIRECT_URI` | no | `https://<PROD_DOMAIN>/auth/callback`; must be registered on the OIDC client |
| `CONTESTDOJO_EVENT_ID` | no | the event this puzzle round is scoped to |
| `SESSION_SECRET` | **yes** | Signs the session cookie (HMAC). **Must be a fixed, stable value shared identically across all replicas**, or sessions break across pods/restarts. Generate once: `openssl rand -hex 32`. (Random per-process if unset.) |

### Optional (sane production defaults; override only if needed)

| Variable | Default |
| --- | --- |
| `OIDC_ISSUER` | `https://contestdojo.com/api/oidc` |
| `CONTESTDOJO_API_BASE` | `https://api.contestdojo.com` |

## Horizontal scaling

**Run a single replica (`replicas: 1`) unless you add room-aware routing.**

The HTTP/SSR surface and single-player solving are stateless and would scale
fine. The **multiplayer realtime layer is not** horizontally scalable as-is:

- Active rooms live in a **process-global in-memory map**
  (`ROOM_MANAGER` in `src/realtime/server.rs`); each room holds the websocket
  clients connected *to that process*, and an edit is broadcast only to those
  local clients.
- A room's state is snapshotted to Postgres every ~5s, but a room is loaded
  from the DB **only once** when first opened on a process and is never
  re-read afterward.

So with more than one replica, two users in the same room that land on
different pods won't see each other's live edits, and the periodic per-pod
snapshots can clobber each other (last-write-wins per room), risking lost
progress.

To run more than one replica you'd need **one** of:

1. **Room-affinity routing** — route every connection for a given room to the
   same pod (consistent hashing on the room/team id at the gateway; e.g. a
   `StatefulSet` + headless `Service` + hash router, or an L7 proxy with
   consistent hashing). Note: plain client-IP/cookie session affinity is *not*
   enough — affinity must be by **room**, since different clients in the same
   room must share a pod.
2. **A shared pub/sub backplane** — broadcast ops across pods (e.g. Postgres
   `LISTEN/NOTIFY` or Redis) with the DB as the single source of truth and
   proper CRDT merge on load. This is an application change (out of scope for
   this image).

## Health check

The image serves `GET /` on port `3000` (returns `200` when up) — use it for
container/orchestrator readiness & liveness probes. The image also defines a
Docker `HEALTHCHECK` hitting the same endpoint.

## Build gotchas

- **Bind address**: the upstream default is `127.0.0.1:3000`. The image
  overrides this to `[::]:3000` via `LEPTOS_SITE_ADDR`; without it the
  container is unreachable from outside. `[::]` is dual-stack on Linux
  (`IPV6_V6ONLY=0` by default), so the server listens on both IPv6 and IPv4.
- **Static site must be present**: both the server binary and `target/site`
  must be in the runtime image. The server serves assets from
  `LEPTOS_SITE_ROOT` (set to `site`, matching where the Dockerfile copies them).
- **Nightly toolchain**: `cargo-leptos` requires nightly + the
  `wasm32-unknown-unknown` target; both are set up in the builder stage.
- **Tailwind**: `cargo-leptos` auto-downloads the standalone Tailwind binary at
  build time, so the builder needs network access (normal for `docker build`).
- **recursion_limit**: the release build raises `recursion_limit` to 256 (a
  compiler directive, no behavior change) so nightly's trait solver doesn't
  overflow (`E0275`) on nested leptos/tachys view tuples.
