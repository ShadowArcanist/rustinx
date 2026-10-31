# Example: Using rustinx in your app's Dockerfile
# This shows how to build a static site and serve it with rustinx

# ── Option A: Multi-stage build (build site + serve) ──────────

# Pin versions. For production, pin by digest: node:22-alpine@sha256:<digest>
FROM node:22-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build

FROM ghcr.io/shadowarcanist/rustinx:v1.1
COPY --from=builder /app/dist /static
COPY rustinx.toml /etc/rustinx/rustinx.toml
# Optional: copy redirect files
# COPY redirects/ /etc/rustinx/redirects/


# ── Option B: Pre-built static files only ─────────────────────
# If your CI builds the static files separately:
#
# FROM ghcr.io/shadowarcanist/rustinx:v1.1
# COPY ./dist /static
# COPY rustinx.toml /etc/rustinx/rustinx.toml


# ── Minimal app config (rustinx.toml) ────────────────────────
# [server]
# listen = "0.0.0.0:9090"
# behind_proxy = true
# trusted_proxy_depth = 2
#
# [[vhost]]
# hostnames = ["_"]
# root = "/static"
# spa = true
