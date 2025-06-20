use std::{collections::HashMap, path, sync::Arc};

use crate::structs::{
    COMMAND_FILE, COMMAND_ME, COMMAND_ONLINE, COMMAND_QUIT, EOF_ERROR, EOF_EVENT, EOF_MESSAGE,
    MAX_FILESIZE, Message, Msg,
};
use crate::utils::{self, now};

use anyhow::Result;
use futures_lite::StreamExt;
use iroh::{Endpoint, NodeId}; // PublicKey
use iroh_gossip::net::{Event, GossipEvent, GossipReceiver, GossipSender};
use tokio::io::{self, AsyncBufReadExt};
use tokio::sync::RwLock;
use tokio::{fs, time};

/// Read input from stdin
pub async fn input_loop(
    endpoint: Endpoint,
    name: String,
    sender: GossipSender,
    members: Arc<RwLock<HashMap<NodeId, String>>>,
) -> Result<()> {
    let node_id: NodeId = endpoint.node_id();
    let eol = &['\r', '\n'][..];

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

        let (command, _) = utils::split_first_space(&text);

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
            COMMAND_FILE => {
                let (_, filename) = utils::split_first_space(&line[COMMAND_FILE.len()..]);
                let filename = match filename {
                    Some(v) => v.trim(),
                    None => {
                        println!("!!! no input file");
                        continue;
                    }
                };
                tokio::spawn(send_file(node_id, sender.clone(), filename.to_string()));
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

pub async fn send_file(node_id: NodeId, sender: GossipSender, filename: String) {
    let filepath = path::Path::new(&filename);

    if !(filepath.exists() && filepath.is_file()) {
        println!("!!! invalid input file");
        return;
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
            println!("!!! Failed to read file: {filename}, {e:?}");
            return;
        }
    };

    if metadata.len() > MAX_FILESIZE {
        println!("!!! File size is large than {MAX_FILESIZE}");
        return;
    }

    println!("--> {} SendingFile: {filename}\n{EOF_EVENT}", now());

    //let content = fs::read(filepath).await.map_err(|e| {
    //    println!("!!! {} Failed to read file: {}, {}", now(), filename, e);
    //    continue;
    //})?;
    let content = match fs::read(&filepath).await {
        Ok(v) => v,
        Err(e) => {
            println!("!!! {} Failed to read file: {filename}, {e:?}\n{EOF_ERROR}", now());
            return;
        }
    };

    let msg = Msg::File { from: node_id, filename: filename.clone(), content };

    match sender.broadcast(msg.to_vec().into()).await {
        Ok(_) => println!("--> {} SentFileOK: {filename}\n{EOF_EVENT}", now()),
        Err(e) => println!("!!! {} SendFileError: {filename}, {e:?}\n{EOF_ERROR}", now()),
    }
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
                if let Some((_, name)) = members.remove_entry(&from) {
                    println!(
                        "<-- NeighborDown: {name:?}, {:?}, {}\n{EOF_EVENT}",
                        now(),
                        from.fmt_short(),
                    );
                } else {
                    println!("<-- {} NeighborDown: UNKNOWN, {}\n{EOF_EVENT}", now(), from);
                }
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
                        println!("<-- {} Bye: {name:?}, {}\n{EOF_EVENT}", now(), from.fmt_short())
                    }
                    None => println!("<-- {} Bye: UNKNOWN, {}\n{EOF_EVENT}", now(), from),
                }
            }
            Msg::AboutMe { from, name, at } => {
                let mut members = members.write().await;
                // if it's an `AboutMe` message add and entry into the map and print the name
                if !members.contains_key(&from) {
                    members.insert(from, name.clone());
                    // println!("<-- Peer: {} is now known as {:?}", from, name);
                    println!("<-- {} NewPeer: {}, {:?}, {}\n{EOF_EVENT}", now(), from, name, at);
                }

                if let Err(e) = sender.broadcast(about_me.to_bytes().into()).await {
                    println!("!!! {} BroadcastError: {e:?}\n{EOF_ERROR}", now());
                }
            }
            Msg::Message { from, text } => {
                let members = members.read().await;
                // if it's a `Message` message, get the name from the map and print the message
                let name = members.get(&from).map_or_else(|| from.fmt_short(), String::to_string);
                println!("<<< {} MsgFrom: {:?}, \n{}\n{EOF_MESSAGE}", now(), name, text.trim_end());
            }
            Msg::File { from, filename, content } => {
                tokio::spawn(save_file(from, name.clone(), filename, content));
            }
        }
    }
    Ok(())
}

async fn save_file(from: NodeId, name: String, filename: String, content: Vec<u8>) {
    let dir = path::Path::new("data").join("downloads");

    let filepath = match path::Path::new(&filename).file_name() {
        Some(v) => v.to_string_lossy().to_string(),
        None => {
            println!("!!!! Invalid filepath: filename");
            return;
        }
    };

    let filepath = dir.join(format!("{}_{}", utils::filename_prefix(), filepath));

    if content.len() > MAX_FILESIZE.try_into().unwrap() {
        println!("!!! File size is large than {MAX_FILESIZE}");
        return;
    }

    if let Err(e) = fs::create_dir_all(dir.clone()).await {
        println!("!!! Failed to create dir: {filename}, {e:?}");
        return;
    }

    println!("<-- {} ReceivedFile: {name:?}, {from}, {filename}\n{EOF_EVENT}", now());

    if let Err(e) = fs::write(&filepath, content).await {
        println!("!!! Failed to write file: {filename}, {e:?}");
        return;
    };

    println!("<-- {} SavedFile: {name:?}, {from}, {filename}\n{EOF_EVENT}", now());
}
