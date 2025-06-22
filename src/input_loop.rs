use std::collections::HashMap;

use crate::structs::{
    COMMAND_ME, COMMAND_ONLINE, COMMAND_QUIT, COMMAND_RECEIVE, COMMAND_SEND, COMMAND_SHARE,
    EOF_EVENT, EOF_MESSAGE, Msg,
};
use crate::transfer::{receive_file, share_file};
use crate::utils::{now, read_file_to_send, split_first_space};

use anyhow::Result;
use iroh::{Endpoint, NodeId, protocol::Router};
use iroh_blobs::{net_protocol::Blobs, ticket::BlobTicket};
use iroh_gossip::net::GossipSender;
use tokio::io::{self, AsyncBufReadExt};
use tokio::{sync::RwLock, time};
use tracing::{error, info, warn}; // Level, instrument

/// Read input from stdin
pub async fn input_loop(
    endpoint: Endpoint,
    name: String,
    sender: GossipSender,
    members: std::sync::Arc<RwLock<HashMap<NodeId, String>>>,
) -> Result<()> {
    let node_id: NodeId = endpoint.node_id();
    let eol = &['\r', '\n'][..];

    // println!("module_path = {}", module_path!());

    // We initialize the Blobs protocol in-memory
    let blobs = Blobs::memory().build(&endpoint);
    // Now we build a router that accepts blobs connections & routes them to the blobs protocol.
    let _router = Router::builder(endpoint).accept(iroh_blobs::ALPN, blobs.clone()).spawn();
    // We use a blobs client to interact with the blobs protocol we're running locally:
    let blobs_client = blobs.client();

    /*
    // create a new string buffer
    let mut buffer = String::new();
    // get a handle on `Stdin`
    let stdin = std::io::stdin(); // We get `Stdin` here.

    loop {
        stdin.read_line(&mut buffer)?; // loop through reading from the buffer...
        if buffer.trim_end_matches(eol).ends_with(' ') {
            buffer.truncate(buffer.trim_end().len());
            buffer.push('\n');
            continue;
        }

        let text = buffer.trim_end().to_string();
        buffer.clear(); // clear the buffer after we've sent the content
     */

    let mut reader = io::BufReader::new(io::stdin()).lines();
    let mut lines = String::new();

    while let Some(line) = reader.next_line().await? {
        if line.trim_end_matches(eol).ends_with(' ') {
            lines.push_str(line.trim_end());
            lines.push('\n');
            continue;
        }

        lines.push_str(&line);
        let text = lines.trim_end().to_string();
        lines.clear();

        let (command, _) = split_first_space(&text, false);

        match command {
            COMMAND_QUIT => {
                let msg = Msg::Bye { from: node_id, at: now() };
                // broadcast the encoded message
                sender.broadcast(msg.to_vec().into()).await?;
                time::sleep(time::Duration::from_millis(100)).await;
                break;
            }
            COMMAND_ME => println!("ME: {node_id}, {name}"),
            COMMAND_ONLINE => {
                let members = members.read().await;

                println!("members:");
                for (node_id, name) in members.iter() {
                    println!("  {node_id}: {name:?}")
                }
                info!("{EOF_EVENT}");
            }
            COMMAND_SEND => {
                // let (filepath, _) = split_first_space(&line[COMMAND_SEND.len()..], true);
                //if filepath.is_empty() {
                //    warn!("no input file\n{EOF_EVENT}");
                //    continue;
                //};

                let filepath = match shell_words::split(&line) {
                    Ok(args) if args.len() == 2 => args[1].clone(),
                    _ => {
                        warn!("expected {COMMAND_SEND} <filepath>\n{EOF_EVENT}");
                        continue;
                    }
                };

                let filename = filepath.to_string();
                let msg = match read_file_to_send(&filename).await {
                    Ok(content) => Msg::File { from: node_id, filename, content },
                    Err(e) => {
                        error!("SendFile: {filepath}, {e:?}\n{EOF_EVENT}");
                        continue;
                    }
                };

                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => info!("--> SendFile: {filepath}\n{EOF_EVENT}"),
                    Err(e) => error!("SendFile: {filepath}, {e:?}\n{EOF_EVENT}"),
                }
            }
            COMMAND_SHARE => {
                //let (filename, _) = split_first_space(&line[COMMAND_SHARE.len()..], true);
                //if filename.is_empty() {
                //    warn!("no input file\n{EOF_EVENT}");
                //    continue;
                //};

                let filename = match shell_words::split(&line) {
                    Ok(args) if args.len() == 2 => args[1].clone(),
                    _ => {
                        warn!("expected {COMMAND_SHARE} <filepath>\n{EOF_EVENT}");
                        continue;
                    }
                };

                // TODO: async, stop sharing
                let (size, ticket) = match share_file(blobs_client, node_id, &filename).await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("ShareFile:\n{filename}, {e:?}\n{EOF_EVENT}");
                        continue;
                    }
                };
                info!("--> ShareFile: size={size}\n{ticket} {filename}\n{EOF_EVENT}");

                let msg =
                    Msg::Share { from: node_id, filename: filename.to_string(), size, ticket };

                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => info!(">>> You({:?})\n{EOF_MESSAGE}", name),
                    Err(e) => error!("BroadcastShare: {e:?}\n{EOF_EVENT}"),
                }
            }
            COMMAND_RECEIVE => {
                //let (ticket, filename) = split_first_space(&line[COMMAND_RECEIVE.len()..], true);

                //let filename = match filename {
                //    Some(v) => v,
                //    None => {
                //        warn!("no filename\n{EOF_EVENT}");
                //        continue;
                //    }
                //};

                let (ticket, filename) = match shell_words::split(&line) {
                    Ok(args) if args.len() == 3 => (args[1].clone(), args[2].clone()),
                    _ => {
                        warn!("expect {COMMAND_RECEIVE} <ticket> <filepath>\n{EOF_EVENT}");
                        continue;
                    }
                };

                let ticket: BlobTicket = match ticket.parse() {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("invalid ticket: {e:?}\n{EOF_EVENT}");
                        continue;
                    }
                };

                match receive_file(blobs_client, ticket, filename.to_string()).await {
                    Ok(v) => info!("<-- ReceivedFile: {filename}\n{v}\n{EOF_EVENT}"),
                    Err(e) => error!("ReceivedFile: {filename}, {e:?}\n{EOF_EVENT}"),
                }
            }
            _ => {
                let msg = Msg::Message { from: node_id, text: text };

                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => info!(">>> You({:?})\n{EOF_MESSAGE}", name),
                    Err(e) => error!("BroadcastMsg: {e:?}\n{EOF_MESSAGE}"),
                }
            }
        }
    }

    Ok(())
}
