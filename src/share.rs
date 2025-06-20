// https://github.com/n0-computer/iroh-blobs/blob/main/examples/transfer.rs
use std::{path, process};

use anyhow::{Result, anyhow};
use iroh::{Endpoint, protocol::Router};
use iroh_blobs::store::{ExportFormat, ExportMode};
use iroh_blobs::{
    net_protocol::Blobs, rpc::client::blobs::WrapOption, ticket::BlobTicket, util::SetTagOption,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Create an endpoint, it allows creating and accepting connections in the iroh p2p world
    let endpoint = Endpoint::builder().discovery_n0().bind().await?;

    // Grab all passed in arguments, the first one is the binary itself, so we skip it.
    let args: Vec<String> = std::env::args().skip(1).collect();

    share_file(endpoint.clone(), args).await?;

    process::exit(0);
    //Ok(())
}

async fn share_file(endpoint: Endpoint, args: Vec<String>) -> Result<()> {
    // Convert to &str, so we can pattern-match easily:
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

    // We initialize the Blobs protocol in-memory
    let blobs = Blobs::memory().build(&endpoint);

    // We use a blobs client to interact with the blobs protocol we're running locally:
    let blobs_client = blobs.client();

    match arg_refs.as_slice() {
        ["send", filename] => {
            // Now we build a router that accepts blobs connections & routes them to the blobs
            // protocol.
            let router =
                Router::builder(endpoint.clone()).accept(iroh_blobs::ALPN, blobs.clone()).spawn();

            let filepath: path::PathBuf = filename.parse()?;
            let filepath = path::absolute(&filepath)?;

            println!("==> Hashing file: {filename}");

            // keep the file in place and link it, instead of copying it into the in-memory blobs
            // database
            let in_place = true;
            let blob = blobs_client
                .add_from_path(filepath, in_place, SetTagOption::Auto, WrapOption::NoWrap)
                .await?
                .finish()
                .await?;

            let node_id = router.endpoint().node_id();
            let ticket = BlobTicket::new(node_id.into(), blob.hash, blob.format)?;

            println!("--> File hashed, ticket: {ticket}");
            // println!("cargo run --example transfer -- receive {ticket} {}", filename.display());

            tokio::signal::ctrl_c().await?;
            router.shutdown().await?;
        }
        ["receive", ticket, filename] => {
            let filepath: path::PathBuf = filename.parse()?;
            let filepath = path::absolute(filepath)?;
            let ticket: BlobTicket = ticket.parse()?;

            println!("==> Starting download: {filename}");

            blobs_client
                .download(ticket.hash(), ticket.node_addr().clone())
                .await?
                .finish()
                .await?;

            println!("--> Finished download, copying to destination: {filename}");

            blobs_client
                .export(ticket.hash(), filepath.clone(), ExportFormat::Blob, ExportMode::Copy)
                .await?
                .finish()
                .await?;

            println!("<-- Finished copying: {filename}.");
        }
        _ => return Err(anyhow!("!!! Couldn't parse command line arguments: {args:?}")),
    }

    // Gracefully shut down the node
    println!("<= Shutting down.");

    Ok(())
}
