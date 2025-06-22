use std::process::Command;

use chrono::{SecondsFormat, Utc};

fn main() {
    // 获取 Git 信息
    let git_commit = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let git_branch = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // let build_time = chrono::Utc::now().to_rfc3339();
    let build_time = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);

    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
    println!("cargo:rustc-env=GIT_COMMIT_HASH={}", git_commit);
    println!("cargo:rustc-env=GIT_BRANCH={}", git_branch);
}
