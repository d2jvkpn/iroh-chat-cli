use std::path;

use anyhow::{Result, anyhow};
use iroh::NodeId;
use iroh_blobs::rpc::client::blobs::{DownloadOutcome, ExportOutcome, MemClient, WrapOption};
use iroh_blobs::store::{ExportFormat, ExportMode};
use iroh_blobs::{ticket::BlobTicket, util::SetTagOption};
use tokio::fs;

pub async fn share_file(
    blobs_client: &MemClient,
    node_id: NodeId,
    filename: &str,
) -> Result<(u64, BlobTicket)> {
    let filepath: path::PathBuf = filename.parse()?;
    let filepath = path::absolute(&filepath)?;

    if !(filepath.exists() && filepath.is_file()) {
        return Err(anyhow!("invalid input file"));
    }

    // println!("==> Hashing file: {filename}");

    // keep the file in place and link it, instead of copying it into the in-memory blobs database
    let in_place = true;
    let blob = blobs_client
        .add_from_path(filepath, in_place, SetTagOption::Auto, WrapOption::NoWrap)
        .await?
        .finish()
        .await?;

    // let node_id = router.endpoint().node_id();
    let ticket = BlobTicket::new(node_id.into(), blob.hash, blob.format)?;

    Ok((blob.size, ticket))
}

pub async fn receive_file(
    blobs_client: &MemClient,
    ticket: BlobTicket,
    filename: String,
) -> Result<u64> {
    let filepath: path::PathBuf = filename.parse()?;
    let filepath = path::absolute(filepath)?;

    if let Some(dir) = filepath.parent() {
        fs::create_dir_all(dir).await?;
    }

    // println!("==> Starting download: {filename}");
    let download_outcome: DownloadOutcome =
        blobs_client.download(ticket.hash(), ticket.node_addr().clone()).await?.finish().await?;

    // println!("--> Finished download, copying to destination: {filename}");
    let _export_outcome: ExportOutcome = blobs_client
        .export(ticket.hash(), filepath.clone(), ExportFormat::Blob, ExportMode::Copy)
        .await?
        .finish()
        .await?;

    // dbg!(&download_outcome);
    Ok(download_outcome.downloaded_size)
}
