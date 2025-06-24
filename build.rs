use std::process::{Command, Output};

use chrono::{DateTime, SecondsFormat, Utc};

fn main() {
    let output_string = |v: Output| String::from_utf8_lossy(&v.stdout).trim().to_string();

    // let build_time = chrono::Local::now().to_rfc3339(); // SecondsFormat::Millis
    let build_time = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    let git_registry = Command::new("git")
        .args(&["config", "--get", "remote.origin.url"])
        .output() // Result<T, E>
        .ok() // Option<T>
        .map(output_string) // Option<T>
        .unwrap_or_else(|| "unknown".to_string());

    let git_branch = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output() // Result<T, E>
        .map(output_string) // Result<T, E>
        .unwrap_or_else(|_| "unknown".to_string());

    let git_status = Command::new("git")
        .args(&["status", "--short"])
        .output()
        .map(output_string)
        .map(|v| if v == "" { "clean".to_string() } else { "dirty".to_string() })
        .unwrap_or_else(|_| "unknown".to_string());

    let git_commit_hash = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .map(output_string)
        .unwrap_or_else(|_| "unknown".to_string());

    let git_commit_time = Command::new("git")
        .args(&["show", "-s", "--format=%cI", "HEAD"])
        .output()
        .ok()
        .map(output_string)
        .and_then(|s| DateTime::parse_from_rfc3339(s.trim()).ok())
        .map(|dt| dt.with_timezone(&Utc).to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_else(|| "unknown".to_string());

    // git_commit_pushed: git diff origin/$git_branch..HEAD --name-status

    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
    println!("cargo:rustc-env=GIT_REGISTRY={}", git_registry);
    println!("cargo:rustc-env=GIT_BRANCH={}", git_branch);
    println!("cargo:rustc-env=GIT_STATUS={}", git_status);
    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", git_commit_hash);
    println!("cargo:rustc-env=GIT_COMMIT_TIME={}", git_commit_time);
}
