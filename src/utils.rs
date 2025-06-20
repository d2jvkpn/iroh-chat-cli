#![allow(dead_code)]
use std::path::Path;

use crate::structs::TopicTicket;

use anyhow::{Result, anyhow};
use iroh::SecretKey;
//use rand::RngCore;
use chrono::{Local, SecondsFormat};
use rand::prelude::*;
use serde_yaml::Value;
use tokio::fs::{self, File};
use tokio::io::AsyncWriteExt;
// use tracing::{error, info, instrument, warn}; // Level
use tracing_appender::{non_blocking::WorkerGuard, rolling}; // non_blocking::NonBlocking
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::{format, time::FormatTime}; // writer::MakeWriterExt

pub fn load_yaml(path: &str) -> Result<Value> {
    let contents = std::fs::read_to_string(path)?;
    let yaml: Value = serde_yaml::from_str(&contents)?;
    Ok(yaml)
}

pub fn config_get<'a>(yaml: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').fold(Some(yaml), |acc, key| acc?.get(key))
}

pub fn now() -> String {
    let now = Local::now();
    return now.to_rfc3339_opts(SecondsFormat::Millis, true);
}

pub fn filename_prefix() -> String {
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

pub async fn write_topic_ticket(ticket: &TopicTicket, filename: &str) -> Result<()> {
    let node_addr = ticket.nodes.last().ok_or_else(|| anyhow!("nodes is empty"))?;

    let dir = Path::new("configs");
    fs::create_dir_all(dir).await?;

    let filepath = dir.join(format!("{}.topic.ticket", filename));
    let mut file = File::create(&filepath).await?;
    //file.write_all(&ticket.to_bytes()).await?;
    file.write_all(&ticket.to_bytes()).await?;
    file.write_all(b"\n").await?;
    // println!("--> node: {node_addr:?}\n    ticket: {ticket}");
    println!("--> node_id: {}", node_addr.node_id);
    println!("    filepath: {}", filepath.display());
    println!("    relay_url: {:?}", node_addr.relay_url());
    println!("    direct_addresses: {:?}", node_addr.direct_addresses().collect::<Vec<_>>());
    println!("    ticket: {ticket}");

    Ok(())
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

struct LogTimer;

impl FormatTime for LogTimer {
    fn format_time(&self, w: &mut format::Writer<'_>) -> std::fmt::Result {
        let now = Local::now();
        write!(w, "{}", now.format("%Y-%m-%dT%H:%M:%S%:z"))
    }
}

pub fn log2file(prefix: &str, level: &str) -> WorkerGuard {
    let appender = rolling::daily("logs", prefix);
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);

    // RUST_LOG=my_app=info,my_app::submod=debug
    // RUST_LOG=tokio=info,my_crate=debug
    // .with_env_filter(EnvFilter::from_default_env())
    //  with_max_level(Level::WARN)
    tracing_subscriber::fmt()
        .with_timer(LogTimer)
        .with_target(false)
        .with_env_filter(EnvFilter::new(level))
        // .with_writer(non_blocking.and(std::io::stdout))
        .with_writer(non_blocking)
        .init();

    guard
}

pub fn log2stdout(level: &str) {
    // RUST_LOG=my_app=info,my_app::submod=debug
    // RUST_LOG=tokio=info,my_crate=debug
    // .with_env_filter(EnvFilter::from_default_env())
    //  with_max_level(Level::WARN)
    tracing_subscriber::fmt()
        .with_timer(LogTimer)
        .with_target(false)
        .with_env_filter(EnvFilter::new(level))
        .init();
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
