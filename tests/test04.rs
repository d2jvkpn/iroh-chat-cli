use iroh_chat_cli::utils;

use futures::{FutureExt, pin_mut};
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::test]
async fn main() {
    utils::log2stdout(EnvFilter::new("info"));

    let cancel_token = CancellationToken::new();

    let task1 = tokio::task::spawn(run_task("task1", cancel_token.clone()));
    let task2 = tokio::task::spawn(run_task("task2", cancel_token.clone()));

    // Wrap them with .fuse() so we don't move them
    let (fuse1, fuse2) = (task1.fuse(), task2.fuse());
    pin_mut!(fuse1, fuse2);

    // Wait for either task to complete
    tokio::select! {
        // _ = task1 => {
        _ = &mut fuse1 => {
            warn!("task1 exited...");
        }
        _ = &mut fuse2 => {
            warn!("task2 exited...");
        }
        _ = tokio::signal::ctrl_c() => {
            println!("");
            error!("--> received Ctrl+C.");
        }
    }
    cancel_token.cancel();

    // Optionally wait for both to fully shutdown
    //let _ = task1_fut.await;
    //let _ = task2_fut.await;
    let _ = tokio::join!(fuse1, fuse2);

    warn!("<== All tasks exited.");
}

async fn run_task(name: &str, cancel_token: CancellationToken) {
    info!("==> {name} started");

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                warn!("<-- {name} received cancellation");
                break;
            }
            _ = sleep(Duration::from_secs(2)) => {
                info!("--> {name} is working...");
            }
        }
    }
}
