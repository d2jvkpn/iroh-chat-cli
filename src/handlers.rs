use std::{collections::HashMap, sync::Arc};

use crate::structs::{
    COMMAND_ME, COMMAND_ONLINE, COMMAND_QUIT, COMMAND_RECEIVE, COMMAND_SEND, COMMAND_SHARE,
    EOF_ERROR, EOF_EVENT, EOF_MESSAGE, Message, Msg,
};
use crate::transfer::{file_msg, receive_file, save_file, send_file};
use crate::utils::{self, now};

use anyhow::Result;
use futures_lite::StreamExt;
use iroh::{Endpoint, NodeId, protocol::Router}; // PublicKey
use iroh_blobs::{net_protocol::Blobs, ticket::BlobTicket};
use iroh_gossip::net::{Event, GossipEvent, GossipReceiver, GossipSender};
use tokio::io::{self, AsyncBufReadExt};
use tokio::{sync::RwLock, time};

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

    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin).lines();
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
                    println!("!!! no input file");
                    continue;
                };

                let msg = match file_msg(node_id, filename.to_string()).await {
                    Some(v) => v,
                    None => continue,
                };

                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => println!("--> {} SentFileOK: {filename}\n{EOF_EVENT}", now()),
                    Err(e) => {
                        println!("!!! {} SendFileError: {filename}, {e:?}\n{EOF_ERROR}", now())
                    }
                }
            }
            COMMAND_SHARE => {
                let (filename, _) = utils::split_first_space(&line[COMMAND_SHARE.len()..], true);
                if filename.is_empty() {
                    println!("!!! no input file");
                    continue;
                };

                // TODO: async, stop sharing
                let ticket = match send_file(blobs_client, node_id, filename.to_string()).await {
                    Ok(v) => v,
                    Err(e) => {
                        println!("!!! {} send_file:\n    {filename}, {e:?}\n{EOF_EVENT}", now());
                        continue;
                    }
                };
                println!("--> {} send_file:\n    {ticket} {filename}\n{EOF_EVENT}", now());

                let msg = Msg::Share { from: node_id, filename: filename.to_string(), ticket };
                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => println!(">>> {} You({:?})\n{EOF_MESSAGE}", now(), name),
                    Err(e) => println!("!!! {} BroadcastError: {e:?}\n{EOF_ERROR}", now()),
                }
            }
            COMMAND_RECEIVE => {
                let (ticket, filename) =
                    utils::split_first_space(&line[COMMAND_RECEIVE.len()..], true);

                let filename = match filename {
                    Some(v) => v,
                    None => {
                        println!("!!! no filename");
                        continue;
                    }
                };

                let ticket: BlobTicket = match ticket.parse() {
                    Ok(v) => v,
                    Err(e) => {
                        println!("!!! invalid ticket: {e:?}");
                        continue;
                    }
                };

                match receive_file(blobs_client, ticket, filename.to_string()).await {
                    Ok(_) => println!("<-- {} ReceivedFile: {filename}", now()),
                    Err(e) => println!("!!! {} ReceivedFile: {filename}, {e:?}", now()),
                }
            }
            _ => {
                let msg = Msg::Message { from: node_id, text: text };
                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => println!(">>> {} You({:?})\n{EOF_MESSAGE}", now(), name),
                    Err(e) => println!("!!! {} BroadcastError: {e:?}\n{EOF_ERROR}", now()),
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

    while let Some(event) = receiver.try_next().await? {
        let msg = match event {
            Event::Lagged => {
                println!("<-- {} Lagged\n{EOF_EVENT}", now());
                continue;
            }
            Event::Gossip(GossipEvent::Joined(node_ids)) => {
                println!("<-- {} Joined: {:?}\n{EOF_EVENT}", now(), node_ids);
                continue;
            }
            Event::Gossip(GossipEvent::NeighborUp(from)) => {
                println!("<-- {} NeighborUp: {from}\n{EOF_EVENT}", now());
                continue;
            }
            Event::Gossip(GossipEvent::NeighborDown(from)) => {
                let mut members = members.write().await;
                let peer =
                    members.remove_entry(&from).unwrap_or_else(|| (from, "UNKNOWN".to_string()));
                println!(
                    "<-- {} NeighborDown: {}, {:?}\n{EOF_EVENT}",
                    now(),
                    from.fmt_short(),
                    peer.1,
                );
                continue;
            }
            Event::Gossip(GossipEvent::Received(msg)) => msg,
        };

        // deserialize the message and match on the message type:
        match Message::from_bytes(&msg.content)?.msg {
            Msg::Bye { from, at: _ } => {
                let mut members = members.write().await;
                match members.remove_entry(&from) {
                    Some((_, name)) => {
                        println!("<-- {} Bye: {}, {name:?}\n{EOF_EVENT}", now(), from.fmt_short());
                    }
                    None => println!("<-- {} Bye: {from}, UNKNOWN\n{EOF_EVENT}", now()),
                }
            }
            Msg::AboutMe { from, name: peer_name, at } => {
                let mut members = members.write().await;
                // if it's an `AboutMe` message add and entry into the map and print the name
                if !members.contains_key(&from) {
                    members.insert(from, peer_name.clone());
                    // println!("<-- Peer: {} is now known as {:?}", from, name);
                    println!("<-- {} NewPeer: {from}\n    {peer_name:?}, {at}\n{EOF_EVENT}", now());
                }

                if let Err(e) = sender.broadcast(about_me.to_bytes().into()).await {
                    println!("!!! {} BroadcastError: {e:?}\n{EOF_ERROR}", now());
                }
            }
            Msg::Message { from, text } => {
                let members = members.read().await;
                // if it's a `Message` message, get the name from the map and print the message
                let peer_name =
                    members.get(&from).map_or_else(|| from.fmt_short(), String::to_string);
                println!(
                    "<<< {} Message: {peer_name:?}\n{}\n{EOF_MESSAGE}",
                    now(),
                    text.trim_end()
                );
            }
            Msg::File { from, filename, content } => {
                let members = members.read().await;
                // if it's a `Message` message, get the name from the map and print the message
                let peer_name =
                    members.get(&from).map_or_else(|| from.fmt_short(), String::to_string);
                tokio::spawn(save_file(from, peer_name, filename, content));
            }
            Msg::Share { from, filename, ticket } => {
                let members = members.read().await;
                // if it's a `Message` message, get the name from the map and print the message
                let peer_name =
                    members.get(&from).map_or_else(|| from.fmt_short(), String::to_string);
                println!(
                    "<<< {} Share: {peer_name:?}\n    {ticket} {filename}\n{EOF_MESSAGE}",
                    now(),
                );
            }
        }
    }
    Ok(())
}
