use std::collections::HashMap;

use crate::structs::{EOF_BLOCK, Message, Msg};
use crate::utils::{content_to_file, now};

use anyhow::Result;
use futures_lite::StreamExt;
use iroh::{NodeId, PublicKey};
use iroh_gossip::net::{self, Event, GossipEvent, GossipReceiver, GossipSender};
use tokio::sync::RwLock;
use tracing::{error, info, warn}; // Level, instrument

pub async fn subscribe_loop(
    name: String,
    sender: GossipSender,
    mut receiver: GossipReceiver,
    members: std::sync::Arc<RwLock<HashMap<NodeId, String>>>,
) -> Result<()> {
    let about_me = Message::new(Msg::AboutMe { name: name.to_string(), at: now() });

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
        let msg: net::Message = match event {
            Event::Lagged => {
                warn!("=== Lagged");
                continue;
            }
            Event::Gossip(GossipEvent::Joined(node_ids)) => {
                info!("=== Joined: {:?}", node_ids);
                continue;
            }
            Event::Gossip(GossipEvent::NeighborUp(from)) => {
                info!("=== NeighborUp: {from}");
                continue;
            }
            Event::Gossip(GossipEvent::NeighborDown(from)) => {
                let entry = remove_entry(&from).await;
                info!("=== NeighborDown: {entry}");
                continue;
            }
            Event::Gossip(GossipEvent::Received(v)) => v,
        };

        let from = msg.delivered_from;
        // dbg!(&from);
        let msg: Msg = match Message::from_bytes(&msg.content) {
            Ok(v) => v.msg,
            Err(e) => {
                error!("Unknown message: {}, {e:?}\n{EOF_BLOCK}", get_entry(&from).await);
                continue;
            }
        };

        // deserialize the message and match on the message type:
        match msg {
            Msg::Bye { at } => {
                let entry = remove_entry(&from).await;
                warn!("<-- Bye: {entry}\n{at}");
            }
            Msg::AboutMe { name: peer_name, at } => {
                let mut members = members.write().await;
                // if it's an `AboutMe` message add and entry into the map and print the name
                if !members.contains_key(&from) {
                    members.insert(from, peer_name.clone());
                    // println!("<-- Peer: {} is now known as {:?}", from, name);
                    info!("<-- NewPeer: {from}\nname={peer_name:?}, at={at}");
                }

                if let Err(e) = sender.broadcast(about_me.to_bytes().into()).await {
                    error!("Broadcast AbountMe: {e:?}");
                }
            }
            Msg::Message { text } => {
                let entry = get_entry(&from).await;
                info!("<<< Message: {entry}\n{}", text.trim_end());
            }
            Msg::SendFile { filename, content } => {
                let entry = get_entry(&from).await;
                // tokio::spawn(save_file(entry, filename, content));
                let size = content.len();

                tokio::spawn(async move {
                    match content_to_file(content, &filename).await {
                        Ok(v) => {
                            info!("<-- Received SendFile: {entry}, {filename}");
                            println!("size={size}, path={v}");
                        }
                        Err(e) => {
                            error!("!!! Received SendFile: {entry}, {filename}, {e:?}");
                        }
                    }
                });
            }
            Msg::ShareFile { filename, size, ticket } => {
                let entry = get_entry(&from).await;
                info!("<-- Got Share: {entry}, size={size}\n{ticket} {filename}");
            }
        }

        println!("{}", EOF_BLOCK);
    }

    Ok(())
}
