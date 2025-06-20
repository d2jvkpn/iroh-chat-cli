use std::process;

use chrono::Local;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::signal;
use tokio::time::{self, Duration};
use tracing::{error, info, instrument, warn}; // Level
use tracing_appender::rolling;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt::{format::Writer, time::FormatTime, writer::MakeWriterExt};

struct LocalTimer;

impl FormatTime for LocalTimer {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let now = Local::now();
        write!(w, "{}", now.format("%Y-%m-%dT%H:%M:%S%:z"))
    }
}

#[instrument]
fn my_func(x: i32) {
    info!("Running my_func with x = {}", x);
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let file_appender = rolling::daily("logs", "a01");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // RUST_LOG=my_app=info,my_app::submod=debug
    // RUST_LOG=tokio=info,my_crate=debug
    // .with_env_filter(EnvFilter::from_default_env())
    // with_max_level(Level::WARN)
    tracing_subscriber::fmt()
        .with_timer(LocalTimer)
        .with_target(false)
        .with_env_filter(EnvFilter::new("info"))
        .with_writer(non_blocking.and(std::io::stdout))
        .init();

    my_func(42);
    warn!("warning message with local time");
    error!("this is error");

    let mut stdin_lines = BufReader::new(io::stdin()).lines();

    println!("==> Type something, or press Ctrl+C to exit.");
    println!("    Auto-exits after 60 seconds of inactivity.");

    loop {
        let line = tokio::select! {
            // If the user types within 60 seconds, read and print
            // maybe_line =  stdin_lines.next_line() => {
            maybe_line = time::timeout(Duration::from_secs(60), stdin_lines.next_line()) => {
               match maybe_line {
                    Ok(v) => v,
                    Err(_) => {
                      warn!("!!! No input received in 60 seconds. Exiting gracefully.");
                      break;
                    }
                }
            }

            // Ctrl+C support
            _ = signal::ctrl_c() => {
                warn!("\nReceived Ctrl+C. Exiting.");
                break;
            }
        };

        match line {
            Ok(Some(v)) => info!(">>> You: {}", v),
            Ok(None) => info!("<== End of input (EOF). Exiting."),
            Err(e) => error!("!!! Error reading input: {e:?}"), /* eprintln!("!!! Error reading
                                                                 * input: {}", e), */
        }
    }

    info!("<== Goodbye!");
    io::stdout().flush().await?;
    process::exit(0);
    // Ok(())
}
