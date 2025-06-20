use std::path;

use crate::structs::{EOF_ERROR, EOF_EVENT, MAX_FILESIZE, Msg};
use crate::utils;

use anyhow::Result;
use iroh::NodeId;
use iroh_blobs::rpc::client::blobs::{MemClient, WrapOption};
use iroh_blobs::store::{ExportFormat, ExportMode};
use iroh_blobs::{ticket::BlobTicket, util::SetTagOption};
use tokio::fs;
use tracing::{error, info, warn}; // Level, instrument

pub async fn file_msg(node_id: NodeId, filename: String) -> Option<Msg> {
    let filepath = path::Path::new(&filename);

    if !(filepath.exists() && filepath.is_file()) {
        warn!("invalid input file: {filename}");
        return None;
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
        Err(e) => {
            error!("failed to read file: {filename}, {e:?}");
            return None;
        }
    };

    if metadata.len() > MAX_FILESIZE {
        error!("file size is large than {MAX_FILESIZE}");
        return None;
    }

    info!("--> SendingFile: {filename}\n{EOF_EVENT}");

    //let content = fs::read(filepath).await.map_err(|e| {
    //    println!("!!! {} Failed to read file: {}, {}", now(), filename, e);
    //    continue;
    //})?;
    let content = match fs::read(&filepath).await {
        Ok(v) => v,
        Err(e) => {
            error!("failed to read file: {filename}, {e:?}\n{EOF_ERROR}");
            return None;
        }
    };

    return Some(Msg::File { from: node_id, filename: filename.clone(), content });
}

pub async fn save_file(source: String, filename: String, content: Vec<u8>) {
    let dir = path::Path::new("data").join("downloads");

    info!("<-- ReceivingFile: {source}, {filename}\n{EOF_EVENT}");

    let filepath = match path::Path::new(&filename).file_name() {
        Some(v) => v.to_string_lossy().to_string(),
        None => {
            error!("invalid filepath: {source}, filename");
            return;
        }
    };

    let filepath = dir.join(format!("{}_{}", utils::filename_prefix(), filepath));

    if content.len() > MAX_FILESIZE.try_into().unwrap() {
        error!("file size is too large: {source}, {MAX_FILESIZE}");
        return;
    }

    if let Err(e) = fs::create_dir_all(dir.clone()).await {
        error!("failed to create dir: {source}, {filename}, {e:?}");
        return;
    }

    if let Err(e) = fs::write(&filepath, content).await {
        error!("failed to write file: {source}, {filename}, {e:?}");
        return;
    };

    info!("<-- SavedFile: {source}, {}\n{EOF_EVENT}", filepath.display());
}

pub async fn send_file(
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
