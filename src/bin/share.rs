use std::path;

use iroh_chat_cli::transfer::{receive_file, share_file};
use iroh_chat_cli::utils;

use anyhow::{Result, anyhow};
use iroh::{Endpoint, protocol::Router};
use iroh_blobs::{net_protocol::Blobs, ticket::BlobTicket};
use tokio::fs;
use tracing::{error, info}; // Level, instrument, warn
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Grab all passed in arguments, the first one is the binary itself, so we skip it.
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Convert to &str, so we can pattern-match easily:
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    let filter = EnvFilter::new(format!("{0}=info", module_path!()));
    utils::log2stdout(filter);

    // Create an endpoint, it allows creating and accepting connections in the iroh p2p world
    let endpoint = Endpoint::builder().discovery_n0().bind().await?;
    let node_id = endpoint.node_id(); // router.endpoint().node_id();

    // We initialize the Blobs protocol in-memory
    let blobs = Blobs::memory().build(&endpoint);
    // Now we build a router that accepts blobs connections & routes them to the blobs protocol.
    let router = Router::builder(endpoint).accept(iroh_blobs::ALPN, blobs.clone()).spawn();
    // We use a blobs client to interact with the blobs protocol we're running locally:
    let blobs_client = blobs.client();

    match arg_refs.as_slice() {
        ["share", filepath] | ["share", filepath, _] => {
            let (size, ticket) = share_file(blobs_client, node_id, filepath).await?;

            let basename = path::Path::new(&filepath)
                .file_name()
                .ok_or(anyhow!("get filename"))
                .map(|v| v.to_string_lossy().to_string())?;

            if arg_refs.len() > 2 {
                let save_path = path::Path::new(arg_refs[2]);
                if let Some(p) = save_path.parent() {
                    fs::create_dir_all(p).await?;
                }

                fs::write(save_path, ticket.to_string()).await?;
                info!(
                    "==> SharingFile: size={size}, {filepath}\n{} {basename}",
                    save_path.display()
                );
            } else {
                info!("==> SharingFile: size={size}, {filepath}\n{ticket} {basename}");
            }

            tokio::signal::ctrl_c().await?;
            println!("");
            // Gracefully shut down the node
            router.shutdown().await?;
        }
        ["receive", ticket, filepath] => {
            let ticket: BlobTicket = if ticket.contains(".") {
                let ticket = fs::read_to_string(&ticket).await?;
                ticket.parse()?
            } else {
                ticket.parse()?
            };

            info!("<-- receiving_file: {filepath}");
            receive_file(blobs_client, ticket, filepath.to_string()).await?;
            info!("<-- received_file: {filepath}");
        }
        _ => {
            error!("couldn't parse command line arguments: {args:?}");
            std::process::exit(1);
        }
    }

    info!("<== Exit");
    std::process::exit(0);
    //Ok(())
}
