use iroh_chat_cli::utils;

use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::time::{Duration, timeout};
use tracing::{error, info, instrument, warn}; // Level
use tracing_subscriber::EnvFilter;

#[instrument]
fn my_func(x: i32) {
    info!("running my_func");

    std::thread::sleep(std::time::Duration::from_millis(42));
    // tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    info!("exit my_func");
}

#[tokio::test]
async fn main() -> io::Result<()> {
    // let _guard = utils::log2file("test01.log", "info");
    utils::log2stdout(EnvFilter::new(format!("{}={}", module_path!(), "info")));

    my_func(42);
    warn!("warning message with local time");
    error!("this is an error");

    let mut stdin_lines = BufReader::new(io::stdin()).lines();

    info!(
        "==> Type something, or press Ctrl+C to exit. Auto-exits after 60 seconds of inactivity."
    );

    loop {
        let line = tokio::select! {
            // If the user types within 60 seconds, read and print
            // maybe_line =  stdin_lines.next_line() => {
            maybe_line = timeout(Duration::from_secs(60), stdin_lines.next_line()) => {
               match maybe_line {
                    Ok(v) => v,
                    Err(_) => {
                      warn!("no input received in 60 seconds, exiting gracefully.");
                      break;
                    }
                }
            }

            // Ctrl+C support
            _ = tokio::signal::ctrl_c() => {
                println!("");
                warn!("received Ctrl+C, exiting.");
                break;
            }
        };

        match line {
            Ok(Some(val)) if &val == "::quit" => break,
            Ok(Some(v)) => info!(">>> you: {}", v),
            Ok(None) => {
                warn!("end of input (EOF), exiting.");
                break;
            }
            // eprintln!("!!! Error reading input: {}", e),
            Err(e) => {
                error!("reading input: {e:?}");
                std::process::exit(1);
            }
        }
    }

    info!("<== Goodbye!");
    io::stdout().flush().await?;
    std::process::exit(0);
    // Ok(())
}
