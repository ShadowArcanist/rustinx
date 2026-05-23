use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hyper::Method;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;

pub struct AccessLogger {
    tx: mpsc::Sender<String>,
    dropped: AtomicU64,
    offset_secs: i32,
}

impl AccessLogger {
    pub async fn new(
        path: &str,
        offset_secs: i32,
    ) -> Result<(Self, tokio::task::JoinHandle<()>), String> {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("create log dir: {e}"))?;
            }
        }
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .map_err(|e| format!("open log {path}: {e}"))?;

        let (tx, rx) = mpsc::channel(4096);
        let writer_handle = tokio::task::spawn(log_writer(file, rx));
        Ok((
            Self {
                tx,
                dropped: AtomicU64::new(0),
                offset_secs,
            },
            writer_handle,
        ))
    }

    pub fn log(&self, ip: &str, method: &Method, path: &str, status: u16, elapsed: Duration) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let ms = elapsed.as_micros() as f64 / 1000.0;
        let timestamp = format_timestamp(ts, self.offset_secs);
        let line = format!(
            "[{timestamp}] {} {method} {} {status} {ms:.1}ms\n",
            sanitize(ip),
            sanitize(path),
        );
        if self.tx.try_send(line).is_err() {
            let count = self.dropped.fetch_add(1, Ordering::Relaxed) + 1;
            if count == 1 || count.is_multiple_of(1000) {
                eprintln!("[rustinx] log channel full, {count} entries dropped");
            }
        }
    }
}

async fn log_writer(mut file: tokio::fs::File, mut rx: mpsc::Receiver<String>) {
    let mut write_errors: u64 = 0;
    while let Some(line) = rx.recv().await {
        if file.write_all(line.as_bytes()).await.is_err() {
            write_errors += 1;
            if write_errors == 1 || write_errors.is_multiple_of(1000) {
                eprintln!("[rustinx] log write error ({write_errors} total)");
            }
        }
        while let Ok(more) = rx.try_recv() {
            if file.write_all(more.as_bytes()).await.is_err() {
                write_errors += 1;
            }
        }
    }
    if let Err(e) = file.flush().await {
        eprintln!("[rustinx] log flush error: {e}");
    }
    if let Err(e) = file.sync_data().await {
        eprintln!("[rustinx] log sync error: {e}");
    }
}

fn format_timestamp(secs: u64, offset_secs: i32) -> String {
    let adjusted = secs as i64 + offset_secs as i64;

    let day_secs = adjusted.rem_euclid(86400) as u32;
    let hour = day_secs / 3600;
    let minute = (day_secs % 3600) / 60;

    let days = adjusted.div_euclid(86400) as i32;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };

    let (h12, ampm) = match hour {
        0 => (12, "AM"),
        1..=11 => (hour, "AM"),
        12 => (12, "PM"),
        _ => (hour - 12, "PM"),
    };

    format!("{:02}:{:02}{} | {:02}/{:02}", h12, minute, ampm, d, m)
}

pub fn parse_timezone(tz: &str) -> Result<i32, String> {
    let tz = tz.trim();
    if tz == "UTC" || tz == "+00:00" {
        return Ok(0);
    }
    if tz.len() == 6 && (tz.starts_with('+') || tz.starts_with('-')) {
        let sign = if tz.starts_with('-') { -1 } else { 1 };
        let hours: i32 = tz[1..3]
            .parse()
            .map_err(|_| format!("invalid timezone hours: '{tz}'"))?;
        let minutes: i32 = tz[4..6]
            .parse()
            .map_err(|_| format!("invalid timezone minutes: '{tz}'"))?;
        if hours > 14 || minutes > 59 {
            return Err(format!("timezone offset out of range: '{tz}'"));
        }
        return Ok(sign * (hours * 3600 + minutes * 60));
    }
    Err(format!(
        "timezone must be 'UTC' or offset like '+05:30' or '-05:00', got '{tz}'"
    ))
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { '_' } else { c })
        .collect()
}
