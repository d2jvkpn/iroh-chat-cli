use iroh_chat_cli::utils;

use futures::{FutureExt, pin_mut};
use tokio::signal::{self, unix};
use tokio::time::{Duration, sleep};
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::test]
async fn main() {
    utils::log2stdout(EnvFilter::new("info"));

    let mut sigint =
        unix::signal(unix::SignalKind::interrupt()).expect("failed to set up SIGINT handler");

    let mut sigterm =
        unix::signal(unix::SignalKind::terminate()).expect("failed to set up SIGTERM handler");

    let cancel_token = CancellationToken::new();

    let task1 = tokio::task::spawn(run_task("task1", cancel_token.clone()));
    let task2 = tokio::task::spawn(run_task("task2", cancel_token.clone()));

    // Wrap them with .fuse() so we don't move them
    let (fuse1, fuse2) = (task1.fuse(), task2.fuse());
    pin_mut!(fuse1, fuse2);

    info!("==> Starting tasks: pid={}...", std::process::id());
    // Wait for either task to complete
    let (ans1, ans2) = tokio::select! {
        // _ = task1 => {
        ans1 = &mut fuse1 => {
            warn!("task1 exited.");
            warn!("--> cancel token");
            cancel_token.cancel();
            (ans1, fuse2.await)
        }
        ans2 = &mut fuse2 => {
            warn!("task2 exited.");
            warn!("--> cancel token");
            cancel_token.cancel();
            (fuse1.await, ans2)
        }
        _ = signal::ctrl_c() => {
            println!("");
            error!("<-- received Ctrl+C.");
            warn!("--> cancel token");
            cancel_token.cancel();
            info!("<-- waitting for both to fully shutdown.");
            (fuse1.await, fuse2.await)
        }
        _ = sigint.recv() => {
            println!("");
            error!("<-- received SIGINT (Ctrl+C)");
        }
        _ = sigterm.recv() => {
            println!("");
            error!("<-- received SIGTERM (kill)");
            warn!("--> cancel token");
            cancel_token.cancel();
            info!("<-- waitting for both to fully shutdown.");
            (fuse1.await, fuse2.await)
        }
    };

    // Optionally wait for both to fully shutdown
    // let (ans1, ans2) = tokio::join!(task1.await, task2.await);
    info!("<-- answers: task1={ans1:?}, task2={ans2:?}.");

    warn!("<== All tasks exited.");
    std::process::exit(0);
}

async fn run_task(name: &str, cancel_token: CancellationToken) -> u32 {
    info!("--> {name} started...");
    let mut count = 0;

    loop {
        tokio::select! {
            _ = cancel_token.cancelled() => {
                warn!("<-- {name} received cancellation.");
                return count;
            }
            _ = sleep(Duration::from_secs(2)) => {
                count += 1;
                info!("--> {name} is working: {count:03}/inf.");
            }
        }
    }
}
