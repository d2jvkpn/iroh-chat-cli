use std::{process, thread};

use iroh_chat_cli::utils;

use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::signal;
use tokio::time::{self, Duration};
use tracing::{error, info, instrument, warn}; // Level

#[instrument]
fn my_func(x: i32) {
    info!("running my_func with x = {}", x);

    thread::sleep(std::time::Duration::from_millis(42));

    info!("exit my_func");
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let _guard = utils::log2file("test01.log", "info");
    // utils::log2stdout("info");

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
            maybe_line = time::timeout(Duration::from_secs(60), stdin_lines.next_line()) => {
               match maybe_line {
                    Ok(v) => v,
                    Err(_) => {
                      warn!("no input received in 60 seconds, exiting gracefully.");
                      break;
                    }
                }
            }

            // Ctrl+C support
            _ = signal::ctrl_c() => {
                println!("");
                warn!("received Ctrl+C, exiting.");
                break;
            }
        };

        match line {
            Ok(Some(v)) => info!(">>> you: {}", v),
            Ok(None) => info!("end of input (EOF), exiting."),
            Err(e) => error!("reading input: {e:?}"), /* eprintln!("!!! Error reading
                                                       * input: {}", e), */
        }
    }

    info!("<== Goodbye!");
    io::stdout().flush().await?;
    process::exit(0);
    // Ok(())
}
