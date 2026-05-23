# ── Build stage ──────────────────────────────────────────────
# Compile Rustinx with musl for a fully static binary
FROM rust:1.87-alpine AS builder
RUN apk add --no-cache musl-dev
WORKDIR /build
COPY app/Cargo.toml app/Cargo.lock* ./
COPY app/src/ src/
RUN cargo build --release

# ── Runtime stage ────────────────────────────────────────────
# Scratch = no OS, no shell, no package manager, minimal attack surface
FROM scratch
COPY --from=builder /build/target/release/rustinx /usr/bin/rustinx
EXPOSE 9090
# Run as unprivileged user (nobody:nobody)
USER 65534:65534
ENTRYPOINT ["/usr/bin/rustinx"]
CMD ["--config", "/etc/rustinx/rustinx.toml"]
