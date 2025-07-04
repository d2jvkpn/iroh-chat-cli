use std::path;

use crate::structs::MAX_FILESIZE;

use anyhow::{Result, anyhow};
use iroh::SecretKey;
//use rand::RngCore;
use chrono::{DateTime, Local, SecondsFormat, Utc};
use rand::prelude::*;
use serde_yaml::Value;
use tokio::fs;
// use tracing::{error, info, instrument, warn}; // Level
use tracing_appender::{non_blocking::WorkerGuard, rolling}; // non_blocking::NonBlocking
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::{self, time::FormatTime}; // writer::MakeWriterExt

const _BUILD_INFO: &str = concat!(
    "\nBuildInfo:",
    "\n  build_time: ",
    env!("BUILD_TIME"),
    "\n  version: ",
    env!("CARGO_PKG_VERSION"),
    "\n  git_registry: ",
    env!("GIT_REGISTRY"),
    "\n  git_branch: ",
    env!("GIT_BRANCH"),
    "\n  git_status: ",
    env!("GIT_STATUS"),
    "\n  git_commit_hash: ",
    env!("GIT_COMMIT_HASH"),
    "\n  git_commit_time: ",
    env!("GIT_COMMIT_TIME"),
    "\n",
);

pub fn build_info() -> String {
    format!(
        r#"BuildInfo:
  build_time     : {}
  version        : {}
  git_registry   : {}
  git_branch     : {}
  git_status     : {}
  git_commit_hash: {}
  git_commit_time: {}"#,
        env!("BUILD_TIME"),
        env!("CARGO_PKG_VERSION"),
        env!("GIT_REGISTRY"),
        env!("GIT_BRANCH"),
        env!("GIT_STATUS"),
        env!("GIT_COMMIT_HASH"),
        env!("GIT_COMMIT_TIME"),
    )
}

pub fn load_yaml(filepath: &str) -> Result<Value> {
    let contents = std::fs::read_to_string(filepath)?;
    let yaml: Value = serde_yaml::from_str(&contents)?;
    Ok(yaml)
}

pub fn config_get<'a>(value: &'a Value, sub: &str) -> Option<&'a Value> {
    sub.split('.').fold(Some(value), |acc, key| acc?.get(key))
}

pub fn local_now() -> String {
    let now = Local::now();
    return now.to_rfc3339_opts(SecondsFormat::Millis, true);
}

pub fn local_from_millis(millis: i64) -> Result<DateTime<Local>> {
    let secs: i64 = millis / 1_000;
    let nanos: u32 = (millis % 1_000) as u32 * 1_000_000;

    DateTime::from_timestamp(secs, nanos)
        .map(|v| v.with_timezone(&Local))
        .ok_or_else(|| anyhow!("invalid timestamp"))
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
    fn format_time(&self, w: &mut fmt::format::Writer<'_>) -> std::fmt::Result {
        let now = Local::now();
        // write!(w, "{}", now.format("%Y-%m-%dT%H:%M:%S%:z"))
        write!(w, "{}", now.to_rfc3339_opts(SecondsFormat::Millis, true))
    }
}

// EnvFilter::new("info"), EnvFilter::new("myapp=info")
pub fn log2stdout(filter: EnvFilter) {
    // RUST_LOG=my_app=info,my_app::submod=debug
    // RUST_LOG=tokio=info,my_crate=debug
    // .with_env_filter(EnvFilter::from_default_env())
    //  with_max_level(Level::WARN)

    tracing_subscriber::fmt().with_timer(LogTime).with_target(false).with_env_filter(filter).init();
}

pub fn log2file(app: &str, filter: EnvFilter) -> WorkerGuard {
    let appender = rolling::daily("logs", app);
    let (non_blocking, guard) = tracing_appender::non_blocking(appender);

    tracing_subscriber::fmt()
        .with_timer(LogTime)
        .with_target(false)
        .with_env_filter(filter)
        .with_writer(non_blocking) // non_blocking.and(std::io::stdout)
        .init();

    guard
}

pub async fn read_file_content(filename: &str, max_size: u64) -> Result<Vec<u8>> {
    let filepath = path::Path::new(&filename);

    if !filepath.exists() {
        return Err(anyhow!("file not exists"));
    }

    if !filepath.is_file() {
        return Err(anyhow!("not a file"));
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

    if metadata.len() > max_size {
        return Err(anyhow!("file size is large than {max_size}"));
    }

    // info!("--> SendingFile: {filename}\n{EOF_EVENT}");

    //let content = fs::read(filepath).await.map_err(|e| {
    //    println!("!!! {} Failed to read file: {}, {}", now(), filename, e);
    //    continue;
    //})?;

    fs::read(&filepath).await.map_err(|e| anyhow!("failed to read file, {e:?}"))
}

pub async fn content_to_file(content: Vec<u8>, filename: &str) -> Result<String> {
    if content.len() > MAX_FILESIZE.try_into().unwrap() {
        return Err(anyhow!("file size is too large than {MAX_FILESIZE}"));
    }

    // info!("<-- ReceivingFile: {source}, {filename}\n{EOF_EVENT}");
    let filename = match path::Path::new(filename).file_name() {
        Some(v) => v.to_string_lossy().to_string(),
        None => return Err(anyhow!("invalid filepath")),
    };

    let home_dir = std::env::home_dir().ok_or(anyhow!("can't get home dir"))?;

    // let prefix = Local::now().format("%Y-%m-%d-%s").to_string();
    let dir = path::Path::new(&home_dir) // path::Path::new("data").join("received_files")
        .join("apps")
        .join("data")
        .join("iroh-chat-cli")
        .join(Utc::now().format("%Y-%m-%d-utc").to_string());

    let filepath = dir.join(filename);

    fs::create_dir_all(dir.clone()).await.map_err(|e| anyhow!("failed to create dir: {e:?}"))?;
    fs::write(&filepath, content).await.map(|e| anyhow!("failed to write file: {e:?}"))?;

    Ok(format!("{}", filepath.display()))
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

    #[test]
    fn sign() {
        let secret_key = iroh_secret_key();
        let pub_key = secret_key.public();

        let bts = b"Hello, world";
        let signature = secret_key.sign(bts);
        let result = pub_key.verify(bts, &signature);
        assert!(result.is_ok());

        println!("--> signature: {signature}");
    }

    #[test]
    fn time() {
        let at = Utc::now().timestamp_millis();
        println!("{:?}, {:?}", at.to_be_bytes(), 0_i32.to_be_bytes());
    }
}
