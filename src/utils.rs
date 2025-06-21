use std::path;

use crate::structs::MAX_FILESIZE;

use anyhow::{Result, anyhow};
use iroh::SecretKey;
//use rand::RngCore;
use chrono::{Local, SecondsFormat};
use rand::prelude::*;
use serde_yaml::Value;
use tokio::fs;
// use tracing::{error, info, instrument, warn}; // Level
use tracing_appender::{non_blocking::WorkerGuard, rolling}; // non_blocking::NonBlocking
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::{format, time::FormatTime}; // writer::MakeWriterExt

pub fn load_yaml(path: &str) -> Result<Value> {
    let contents = std::fs::read_to_string(path)?;
    let yaml: Value = serde_yaml::from_str(&contents)?;
    Ok(yaml)
}

pub fn config_get<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').fold(Some(value), |acc, key| acc?.get(key))
}

pub fn now() -> String {
    let now = Local::now();
    return now.to_rfc3339_opts(SecondsFormat::Millis, true);
}

pub fn timestamp_prefix() -> String {
    let now = Local::now();
    return now.format("%Y-%m-%d-%s").to_string();
}

pub fn iroh_secret_key() -> SecretKey {
    // let secret_key = SecretKey::generate(rand::rngs::ThreadRng); // !!! rand 0.8
    // let endpoint =
    // Endpoint::builder().secret_key(secret_key.clone()).discovery_n0().bind().await?;
    // dbg!(&secret_key);

    // let yaml = load_yaml(&args.config).await?;
    // let secret_key = config_get(&yaml, "iroh.secret_key").and_then(|v| v.as_str()).unwrap();
    // let secret_key = SecretKey::from_str(secret_key).unwrap();
    // let endpoint = Endpoint::builder().secret_key(secret_key).discovery_n0().bind().await?;

    let mut rng = rand::rng();
    let mut buf = [0u8; 32];
    rng.fill_bytes(&mut buf);

    SecretKey::from_bytes(&buf)
}

pub fn split_first_space(mut s: &str, trim: bool) -> (&str, Option<&str>) {
    if trim {
        s = s.trim();
    }
    match s.split_once(' ') {
        Some((first, rest)) => (first, Some(rest)),
        None => (s, None), // when no space in s
    }
}

struct LogTime;

impl FormatTime for LogTime {
    fn format_time(&self, w: &mut format::Writer<'_>) -> std::fmt::Result {
        let now = Local::now();
        write!(w, "{}", now.format("%Y-%m-%dT%H:%M:%S%:z"))
    }
}

pub fn log2stdout(level: &str) {
    // RUST_LOG=my_app=info,my_app::submod=debug
    // RUST_LOG=tokio=info,my_crate=debug
    // .with_env_filter(EnvFilter::from_default_env())
    //  with_max_level(Level::WARN)
    tracing_subscriber::fmt()
        .with_timer(LogTime)
        .with_target(false)
        .with_env_filter(EnvFilter::new(level))
        .init();
}

pub fn log2file(prefix: &str, level: &str) -> WorkerGuard {
    let appender = rolling::daily("logs", prefix);
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);

    tracing_subscriber::fmt()
        .with_timer(LogTime)
        .with_target(false)
        .with_env_filter(EnvFilter::new(level))
        .with_writer(non_blocking) // non_blocking.and(std::io::stdout)
        .init();

    guard
}

pub async fn read_file_to_send(filename: &str) -> Result<Vec<u8>> {
    let filepath = path::Path::new(&filename);

    if !(filepath.exists() && filepath.is_file()) {
        return Err(anyhow!("invalid input file"));
    }

    /*
    let filepath = match filepath.file_name() {
        Some(v) => v.to_string_lossy().to_string(),
        None => {
            println!("!!! invalid input file");
            return;
        }
    };
    */

    let metadata =
        fs::metadata(&filepath).await.map_err(|e| anyhow!("failed to read file, {e:?}"))?;

    if metadata.len() > MAX_FILESIZE {
        return Err(anyhow!("file size is large than {MAX_FILESIZE}"));
    }

    // info!("--> SendingFile: {filename}\n{EOF_EVENT}");

    //let content = fs::read(filepath).await.map_err(|e| {
    //    println!("!!! {} Failed to read file: {}, {}", now(), filename, e);
    //    continue;
    //})?;

    fs::read(&filepath).await.map_err(|e| anyhow!("failed to write file, {e:?}"))
}

pub async fn content_to_file(content: Vec<u8>, filename: &str) -> Result<()> {
    let dir = path::Path::new("data").join("downloads");

    // info!("<-- ReceivingFile: {source}, {filename}\n{EOF_EVENT}");
    let filepath = match path::Path::new(filename).file_name() {
        Some(v) => v.to_string_lossy().to_string(),
        None => return Err(anyhow!("invalid filepath")),
    };

    let filepath = dir.join(format!("{}_{}", timestamp_prefix(), filepath));

    if content.len() > MAX_FILESIZE.try_into().unwrap() {
        return Err(anyhow!("file size is too large than {MAX_FILESIZE}"));
    }

    fs::create_dir_all(dir.clone()).await.map_err(|e| anyhow!("failed to create dir, {e:?}"))?;
    fs::write(&filepath, content).await.map(|e| anyhow!("failed to write file, {e:?}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_config() {
        let yaml = load_yaml("configs/local.yaml").unwrap();
        let secret_key = config_get(&yaml, "iroh.secret_key").and_then(|v| v.as_str()).unwrap();
        dbg!(&secret_key);
    }
}
