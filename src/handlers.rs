use std::{collections::HashMap, sync::Arc};

use crate::structs::{
    COMMAND_ME, COMMAND_ONLINE, COMMAND_QUIT, COMMAND_RECEIVE, COMMAND_SEND, COMMAND_SHARE,
    EOF_ERROR, EOF_EVENT, EOF_MESSAGE, Message, Msg,
};
use crate::transfer::{receive_file, save_file, send_file, share_file};
use crate::utils::{self, now};

use anyhow::Result;
use futures_lite::StreamExt;
use iroh::{Endpoint, NodeId, PublicKey, protocol::Router};
use iroh_blobs::{net_protocol::Blobs, ticket::BlobTicket};
use iroh_gossip::net::{Event, GossipEvent, GossipReceiver, GossipSender};
use tokio::io::{self, AsyncBufReadExt};
use tokio::{sync::RwLock, time};
use tracing::{error, info, warn}; // Level, instrument

/// Read input from stdin
pub async fn input_loop(
    endpoint: Endpoint,
    name: String,
    sender: GossipSender,
    members: Arc<RwLock<HashMap<NodeId, String>>>,
) -> Result<()> {
    let node_id: NodeId = endpoint.node_id();
    let eol = &['\r', '\n'][..];

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
    let mut buffer = String::new();

    while let Some(line) = reader.next_line().await? {
        if line.trim_end_matches(eol).ends_with(' ') {
            buffer.push_str(line.trim_end());
            buffer.push('\n');
            continue;
        }

        buffer.push_str(&line);
        let text = buffer.trim_end().to_string();
        buffer.clear();

        let (command, _) = utils::split_first_space(&text, false);

        match command {
            COMMAND_QUIT => {
                let msg = Msg::Bye { from: node_id, at: now() };
                // broadcast the encoded message
                sender.broadcast(msg.to_vec().into()).await?;
                time::sleep(time::Duration::from_millis(100)).await;
                break;
            }
            COMMAND_ME => println!("ME: {}, {}", name, node_id),
            COMMAND_ONLINE => {
                let members = members.read().await;
                println!("members:");
                for (node_id, name) in members.iter() {
                    println!("  {node_id}: {name:?}")
                }
            }
            COMMAND_SEND => {
                let (filename, _) = utils::split_first_space(&line[COMMAND_SEND.len()..], true);
                if filename.is_empty() {
                    warn!("no input file");
                    continue;
                };

                let msg = match send_file(node_id, filename.to_string()).await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("SendFile: filename, {e:?}\n{EOF_ERROR}");
                        continue;
                    }
                };

                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => info!("--> SendFile: {filename}\n{EOF_EVENT}"),
                    Err(e) => error!("SendFile: {filename}, {e:?}\n{EOF_ERROR}"),
                }
            }
            COMMAND_SHARE => {
                let (filename, _) = utils::split_first_space(&line[COMMAND_SHARE.len()..], true);
                if filename.is_empty() {
                    warn!("no input file");
                    continue;
                };

                // TODO: async, stop sharing
                let ticket = match share_file(blobs_client, node_id, filename.to_string()).await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("ShareFile:\n{filename}, {e:?}\n{EOF_EVENT}");
                        continue;
                    }
                };
                info!("--> ShareFile:\n{ticket} {filename}\n{EOF_EVENT}");

                let msg = Msg::Share { from: node_id, filename: filename.to_string(), ticket };
                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => info!(">>> You({:?})\n{EOF_MESSAGE}", name),
                    Err(e) => error!("BroadcastError: {e:?}\n{EOF_ERROR}"),
                }
            }
            COMMAND_RECEIVE => {
                let (ticket, filename) =
                    utils::split_first_space(&line[COMMAND_RECEIVE.len()..], true);

                let filename = match filename {
                    Some(v) => v,
                    None => {
                        warn!("no filename");
                        continue;
                    }
                };

                let ticket: BlobTicket = match ticket.parse() {
                    Ok(v) => v,
                    Err(e) => {
                        error!("invalid ticket: {e:?}");
                        continue;
                    }
                };

                match receive_file(blobs_client, ticket, filename.to_string()).await {
                    Ok(_) => info!("<-- ReceivedFile: {filename}"),
                    Err(e) => error!("ReceivedFile: {filename}, {e:?}"),
                }
            }
            _ => {
                let msg = Msg::Message { from: node_id, text: text };
                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => info!(">>> You({:?})\n{EOF_MESSAGE}", name),
                    Err(e) => error!("BroadcastError: {e:?}\n{EOF_ERROR}"),
                }
            }
        }
    }

    Ok(())
}

pub async fn subscribe_loop(
    endpoint: Endpoint,
    name: String,
    sender: GossipSender,
    mut receiver: GossipReceiver,
    members: Arc<RwLock<HashMap<NodeId, String>>>,
) -> Result<()> {
    let node_id: NodeId = endpoint.node_id();
    let about_me = Message::new(Msg::AboutMe { from: node_id, name: name.to_string(), at: now() });

    let get_entry = async |from: &PublicKey| {
        // if it's a `Message` message, get the name from the map and print the message
        members
            .read()
            .await
            .get(from)
            .map(|v| format!("{}({v:?})", from.fmt_short()))
            .unwrap_or_else(|| format!("{from}"))
    };

    let remove_entry = async |from: &PublicKey| {
        members
            .write()
            .await
            .remove_entry(from)
            .map(|v| format!("{}({:?})", v.0.fmt_short(), v.1))
            .unwrap_or_else(|| format!("{from}"))
    };

    while let Some(event) = receiver.try_next().await? {
        let msg = match event {
            Event::Lagged => {
                warn!("<-- Lagged\n{EOF_EVENT}");
                continue;
            }
            Event::Gossip(GossipEvent::Joined(node_ids)) => {
                info!("<-- Joined: {:?}\n{EOF_EVENT}", node_ids);
                continue;
            }
            Event::Gossip(GossipEvent::NeighborUp(from)) => {
                info!("<-- NeighborUp: {from}\n{EOF_EVENT}");
                continue;
            }
            Event::Gossip(GossipEvent::NeighborDown(from)) => {
                let entry = remove_entry(&from).await;
                info!("<-- NeighborDown: {entry}\n{EOF_EVENT}");
                continue;
            }
            Event::Gossip(GossipEvent::Received(msg)) => msg,
        };

        // deserialize the message and match on the message type:
        match Message::from_bytes(&msg.content)?.msg {
            Msg::Bye { from, at: _ } => {
                let entry = remove_entry(&from).await;
                warn!("<-- Bye: {entry}\n{EOF_EVENT}");
            }
            Msg::AboutMe { from, name: peer_name, at } => {
                let mut members = members.write().await;
                // if it's an `AboutMe` message add and entry into the map and print the name
                if !members.contains_key(&from) {
                    members.insert(from, peer_name.clone());
                    // println!("<-- Peer: {} is now known as {:?}", from, name);
                    info!("<-- NewPeer: {from}\n{peer_name:?}, {at}\n{EOF_EVENT}");
                }

                if let Err(e) = sender.broadcast(about_me.to_bytes().into()).await {
                    error!("BroadcastError: {e:?}\n{EOF_ERROR}");
                }
            }
            Msg::Message { from, text } => {
                let entry = get_entry(&from).await;
                info!("<<< Message: {entry}\n{}\n{EOF_MESSAGE}", text.trim_end());
            }
            Msg::File { from, filename, content } => {
                let entry = get_entry(&from).await;
                // tokio::spawn(save_file(entry, filename, content));
                tokio::spawn(async move {
                    match save_file(filename.clone(), content).await {
                        Ok(_) => info!("<-- SavedFile: {entry}, {filename}\n{EOF_EVENT}"),
                        Err(e) => error!("SaveFile: {entry}, {filename}, {e:?}\n{EOF_EVENT}"),
                    }
                });
            }
            Msg::Share { from, filename, ticket } => {
                let entry = get_entry(&from).await;
                info!("<-- Share: {entry}\n{ticket} {filename}\n{EOF_MESSAGE}");
            }
        }
    }
    Ok(())
}
