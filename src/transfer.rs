use std::path;

use crate::structs::{MAX_FILESIZE, Msg};
use crate::utils;

use anyhow::{Result, anyhow};
use iroh::NodeId;
use iroh_blobs::rpc::client::blobs::{MemClient, WrapOption};
use iroh_blobs::store::{ExportFormat, ExportMode};
use iroh_blobs::{ticket::BlobTicket, util::SetTagOption};
use tokio::fs;

pub async fn send_file(node_id: NodeId, filename: String) -> Result<Msg> {
    let filepath = path::Path::new(&filename);

    if !(filepath.exists() && filepath.is_file()) {
        return Err(anyhow!("invalid input file"));
    }

    /*
    let filepath = match filepath.file_name() {
        Some(v) => v.to_string_lossy().to_string(),
        None => {
            println!("!!! invalid input file");
            return;
        }
    };
    */

    let metadata = match fs::metadata(&filepath).await {
        Ok(v) => v,
        Err(e) => return Err(anyhow!("failed to read file, {e:?}")),
    };

    if metadata.len() > MAX_FILESIZE {
        return Err(anyhow!("file size is large than {MAX_FILESIZE}"));
    }

    // info!("--> SendingFile: {filename}\n{EOF_EVENT}");

    //let content = fs::read(filepath).await.map_err(|e| {
    //    println!("!!! {} Failed to read file: {}, {}", now(), filename, e);
    //    continue;
    //})?;
    let content = match fs::read(&filepath).await {
        Ok(v) => v,
        Err(e) => return Err(anyhow!("failed to read file, {e:?}")),
    };

    return Ok(Msg::File { from: node_id, filename: filename.clone(), content });
}

pub async fn save_file(filename: String, content: Vec<u8>) -> Result<()> {
    let dir = path::Path::new("data").join("downloads");

    // info!("<-- ReceivingFile: {source}, {filename}\n{EOF_EVENT}");

    let filepath = match path::Path::new(&filename).file_name() {
        Some(v) => v.to_string_lossy().to_string(),
        None => return Err(anyhow!("invalid filepath")),
    };

    let filepath = dir.join(format!("{}_{}", utils::filename_prefix(), filepath));

    if content.len() > MAX_FILESIZE.try_into().unwrap() {
        return Err(anyhow!("file size is too large than {MAX_FILESIZE}"));
    }

    if let Err(e) = fs::create_dir_all(dir.clone()).await {
        return Err(anyhow!("failed to create dir, {e:?}"));
    }

    if let Err(e) = fs::write(&filepath, content).await {
        return Err(anyhow!("failed to write file, {e:?}"));
    };

    Ok(())
}

pub async fn share_file(
    blobs_client: &MemClient,
    node_id: NodeId,
    filename: String,
) -> Result<BlobTicket> {
    let filepath: path::PathBuf = filename.parse()?;
    let filepath = path::absolute(&filepath)?;

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

    Ok(ticket)
}

pub async fn receive_file(
    blobs_client: &MemClient,
    ticket: BlobTicket,
    filename: String,
) -> Result<()> {
    let filepath: path::PathBuf = filename.parse()?;
    let filepath = path::absolute(filepath)?;

    if let Some(dir) = filepath.parent() {
        fs::create_dir_all(dir).await?;
    }

    // println!("==> Starting download: {filename}");
    blobs_client.download(ticket.hash(), ticket.node_addr().clone()).await?.finish().await?;
    // println!("--> Finished download, copying to destination: {filename}");

    blobs_client
        .export(ticket.hash(), filepath.clone(), ExportFormat::Blob, ExportMode::Copy)
        .await?
        .finish()
        .await?;

    Ok(())
}
