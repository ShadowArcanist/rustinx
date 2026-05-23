use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

mod config;
mod handler;
mod logging;
mod mime_types;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let config_path = parse_args();

    let cfg = if std::path::Path::new(&config_path).exists() {
        config::Config::load(&config_path).unwrap_or_else(|e| {
            eprintln!("[rustinx] config error: {e}");
            std::process::exit(1);
        })
    } else if config_path == "/etc/rustinx/rustinx.toml" {
        eprintln!("[rustinx] no config file — using defaults (serving /static on :9090)");
        config::Config::default()
    } else {
        eprintln!("[rustinx] config not found: {config_path}");
        std::process::exit(1);
    };

    let vhosts = cfg.resolve().unwrap_or_else(|e| {
        eprintln!("[rustinx] config error: {e}");
        std::process::exit(1);
    });

    let (logger, log_handle) = if cfg.logging.enabled {
        let tz_offset = logging::parse_timezone(&cfg.logging.timezone).unwrap_or_else(|e| {
            eprintln!("[rustinx] config error: {e}");
            std::process::exit(1);
        });
        let (l, h) = logging::AccessLogger::new(&cfg.logging.path, tz_offset)
            .await
            .unwrap_or_else(|e| {
                eprintln!("[rustinx] log error: {e}");
                std::process::exit(1);
            });
        (Some(l), Some(h))
    } else {
        (None, None)
    };

    let conn_timeout = Duration::from_secs(cfg.server.connection_timeout);
    let max_connections = cfg.server.max_connections;

    let state = Arc::new(handler::AppState {
        vhosts,
        logger,
        behind_proxy: cfg.server.behind_proxy,
        trusted_proxy_depth: cfg.server.trusted_proxy_depth,
        max_file_size: cfg.server.max_file_size,
    });

    let addr: SocketAddr = cfg.server.listen.parse().unwrap_or_else(|e| {
        eprintln!(
            "[rustinx] invalid listen address '{}': {e}",
            cfg.server.listen
        );
        std::process::exit(1);
    });

    let listener = TcpListener::bind(addr).await.unwrap_or_else(|e| {
        eprintln!("[rustinx] bind {addr} failed: {e}");
        std::process::exit(1);
    });

    let sem = Arc::new(Semaphore::new(max_connections));

    eprintln!("[rustinx] listening on {addr} (max_conn={max_connections})");
    for v in &state.vhosts {
        eprintln!(
            "[rustinx]   vhost: {} -> {}",
            v.hostnames.join(", "),
            v.root.display()
        );
    }

    let mut conn_drops: u64 = 0;
    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, remote_addr) = match result {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[rustinx] accept error: {e}");
                        continue;
                    }
                };

                let permit = match sem.clone().try_acquire_owned() {
                    Ok(p) => p,
                    Err(_) => {
                        drop(stream);
                        conn_drops += 1;
                        if conn_drops == 1 || conn_drops.is_multiple_of(100) {
                            eprintln!("[rustinx] connection limit reached ({conn_drops} total drops)");
                        }
                        continue;
                    }
                };

                let state = state.clone();
                tokio::task::spawn(async move {
                    let _permit = permit;
                    let io = TokioIo::new(stream);
                    let svc = service_fn(move |req| {
                        let state = state.clone();
                        async move { handler::handle(req, state, remote_addr).await }
                    });

                    let result = tokio::time::timeout(
                        conn_timeout,
                        http1::Builder::new()
                            .max_buf_size(16 * 1024)
                            .timer(TokioTimer::new())
                            .header_read_timeout(Duration::from_secs(10))
                            .keep_alive(false)
                            .serve_connection(io, svc),
                    )
                    .await;

                    if let Ok(Err(e)) = result {
                        let msg = e.to_string();
                        if !msg.contains("connection closed")
                            && !msg.contains("connection reset")
                        {
                            eprintln!("[rustinx] connection error: {e}");
                        }
                    }
                });
            }
            _ = shutdown_signal() => {
                eprintln!("[rustinx] shutting down");
                break;
            }
        }
    }

    // Drain access logs: drop state to close sender, then await writer
    drop(state);
    if let Some(h) = log_handle {
        let _ = tokio::time::timeout(Duration::from_secs(2), h).await;
    }
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut term =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to register SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => {}
            _ = term.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
    }
}

fn parse_args() -> String {
    let args: Vec<String> = std::env::args().collect();
    let mut config_path = String::from("/etc/rustinx/rustinx.toml");
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--config" | "-c" => {
                i += 1;
                if i < args.len() {
                    config_path = args[i].clone();
                } else {
                    eprintln!("--config requires a path");
                    std::process::exit(1);
                }
            }
            "--help" | "-h" => {
                println!("rustinx - ultra-lightweight static web server");
                println!();
                println!("Usage: rustinx [OPTIONS]");
                println!();
                println!("Options:");
                println!("  -c, --config <path>  Config file (default: /etc/rustinx/rustinx.toml)");
                println!("  -v, --version        Print version");
                println!("  -h, --help           Print help");
                std::process::exit(0);
            }
            "--version" | "-v" => {
                println!("rustinx {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(1);
            }
        }
        i += 1;
    }
    config_path
}
