use std::process;

use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::signal;
use tokio::time::{self, Duration};

#[tokio::main]
async fn main() -> io::Result<()> {
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
                      println!("!!! No input received in 60 seconds. Exiting gracefully.");
                      break;
                    }
                }
            }

            // Ctrl+C support
            _ = signal::ctrl_c() => {
                println!("\n!!! Received Ctrl+C. Exiting.");
                break;
            }
        };

        match line {
            Ok(Some(v)) => println!(">>> You: {}", v),
            Ok(None) => println!("<== End of input (EOF). Exiting."),
            Err(e) => eprintln!("!!! Error reading input: {}", e),
        }
    }

    println!("<== Goodbye!");
    io::stdout().flush().await?;
    process::exit(0);
    // Ok(())
}
