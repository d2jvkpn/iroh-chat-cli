use crate::structs::{EOF_BLOCK, MemDB, Message, Msg};
use crate::utils::content_to_file;

use crate::structs::parse_raw_message;

use anyhow::Result;
use futures_lite::StreamExt;
use iroh::PublicKey;
use iroh_gossip::net::{self, Event, GossipEvent, GossipReceiver, GossipSender};
use tracing::{error, info, warn}; // Level, instrument

pub async fn subscribe_loop(
    mem_db: MemDB,
    sender: GossipSender,
    mut receiver: GossipReceiver,
) -> Result<()> {
    let (node_id, name) = mem_db.node();
    let about_me = Message::new(node_id, Msg::AboutMe { name: name.clone() });

    let get_entry = async |from: &PublicKey| {
        // if it's a `Message` message, get the name from the map and print the message
        mem_db
            .members
            .read()
            .await
            .get(from)
            .map(|v| format!("{}({v:?})", from.fmt_short()))
            .unwrap_or_else(|| format!("{from}"))
    };

    let remove_entry = async |from: &PublicKey| {
        mem_db
            .members
            .write()
            .await
            .remove_entry(from)
            .map(|v| format!("{}({:?})", v.0.fmt_short(), v.1))
            .unwrap_or_else(|| format!("{from}"))
    };

    while let Some(event) = receiver.try_next().await? {
        let message: net::Message = match event {
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

        // let from = msg.delivered_from;
        // dbg!(&from);
        // let (from, msg, at) = match Message::from_bytes(&message.content[64..]) {
        let (from, msg, at) = match parse_raw_message(&message.content) {
            Ok(v) => (v.from, v.msg, v.at), // NodeId, Msg
            Err(e) => {
                error!(
                    "Unknown message: delivered_from={}, error={e:?}\n{EOF_BLOCK}",
                    get_entry(&message.delivered_from).await
                );
                continue;
            }
        };

        // deserialize the message and match on the message type:
        match msg {
            Msg::Bye => {
                let entry = remove_entry(&from).await;
                warn!("<-- Bye: {entry}, {at}");
            }
            Msg::AboutMe { name: ref peer_name } => {
                let mut members = mem_db.members.write().await;
                // if it's an `AboutMe` message add and entry into the map and print the name
                if !members.contains_key(&from) {
                    members.insert(from, peer_name.clone());
                    // println!("<-- Peer: {} is now known as {:?}", from, name);
                    info!("<-- NewPeer: {from}\nname={peer_name:?}, at={at}");
                }

                if let Err(e) = sender.broadcast(mem_db.sign_message(&about_me)).await {
                    error!("AboutMe broadcast error: {e:?}");
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

                // tokio::spawn(async move { ... }
                match content_to_file(content, &filename).await {
                    Ok(v) => {
                        info!("<-- Received SendFile: {entry}, {filename}");
                        println!("size={size}, path={v}");
                    }
                    Err(e) => {
                        error!("Received SendFile: {entry}, {filename}, {e:?}");
                    }
                };
            }
            Msg::ShareFile { filename, size, ticket } => {
                let entry = get_entry(&from).await;
                info!("<-- Got ShareFile: {entry}, size={size}\n{ticket} {filename}");
            }
        }

        println!("{}", EOF_BLOCK);
    }

    Ok(())
}
