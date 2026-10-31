# Rustinx

Rustinx is a tiny static web server for containers.

It serves files from `/static` on port `9090` by default.

## Docs

https://rustinx.shadowarcanist.com

## Features

- Static file serving
- Docker-first setup
- Multiple virtual hosts
- Optional SPA fallback
- Redirect rules
- Security headers
- Access logging
- Small Rust binary

## Run with Docker

```bash
docker run -p 9090:9090 -v ./dist:/static:ro ghcr.io/shadowarcanist/rustinx:v1.1
```

Open `http://localhost:9090`.

## Build from source

```bash
cd app
cargo build --release
./target/release/rustinx
```
