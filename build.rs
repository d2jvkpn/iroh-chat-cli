use std::process::Command;

use chrono::{DateTime, SecondsFormat, Utc};

fn main() {
    // let build_time = chrono::Utc::now().to_rfc3339();
    let build_time = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true); // SecondsFormat::Millis

    let git_branch = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .map(|v| String::from_utf8_lossy(&v.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let git_commit_hash = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .ok()
        .map(|v| String::from_utf8_lossy(&v.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let git_commit_time = Command::new("git")
        .args(&["show", "-s", "--format=%cI", "HEAD"])
        .output()
        .ok()
        .map(|v| String::from_utf8_lossy(&v.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let git_commit_time = DateTime::parse_from_rfc3339(&git_commit_time)
        .map(|v| v.with_timezone(&Utc).to_rfc3339_opts(SecondsFormat::Secs, true))
        .unwrap_or_else(|_| "unknown".to_string());

    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
    println!("cargo:rustc-env=GIT_BRANCH={}", git_branch);
    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", git_commit_hash);
    println!("cargo:rustc-env=GIT_COMMIT_TIME={}", git_commit_time);
}
