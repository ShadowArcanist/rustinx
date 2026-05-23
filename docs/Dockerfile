# ── Build stage ──────────────────────────────────────────────
# Build Fumadocs site with Bun
FROM oven/bun:1-alpine AS builder
WORKDIR /app

# Install deps without running postinstall (needs source files)
COPY package.json bun.lock ./
RUN bun install --frozen-lockfile --ignore-scripts

# Copy source, then run postinstall + build
COPY . .
RUN bun run postinstall && bun run build

# ── Runtime stage ────────────────────────────────────────────
# Serve fully prerendered static docs with Rustinx
FROM ghcr.io/shadowarcanist/rustinx:v1.0
COPY --from=builder /app/.output/public /static
COPY rustinx.toml /etc/rustinx/rustinx.toml
