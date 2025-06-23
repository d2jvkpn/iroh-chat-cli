use std::{collections::HashMap, process::Command};

use crate::structs::{
    COMMAND_COMMAND, COMMAND_ME, COMMAND_MEMBERS, COMMAND_QUIT, COMMAND_RECEIVE_FILE,
    COMMAND_SEND_FILE, COMMAND_SHARE_FILE, EOF_BLOCK, Msg,
};
use crate::transfer::{receive_file, share_file};
use crate::utils::{now, read_file_to_send, split_first_space};

use anyhow::Result;
use iroh::{Endpoint, NodeId, RelayMap, RelayMode, protocol::Router};
use iroh_blobs::{net_protocol::Blobs, ticket::BlobTicket};
use iroh_gossip::net::GossipSender;
use tokio::io::{self, AsyncBufReadExt};
use tokio::{sync::RwLock, time};
use tracing::{error, info, warn}; // Level, instrument

/// Read input from stdin
pub async fn input_loop(
    node: (NodeId, String),
    sender: GossipSender,
    members: std::sync::Arc<RwLock<HashMap<NodeId, String>>>,
    relay_map: RelayMap,
) -> Result<()> {
    let (node_id, name) = node;
    let eol = &['\r', '\n'][..];
    // println!("module_path = {}", module_path!());

    let blobs_endpoint =
        Endpoint::builder().relay_mode(RelayMode::Custom(relay_map)).discovery_n0().bind().await?;
    let blobs_node_id = blobs_endpoint.node_id(); // router.endpoint().node_id();
    // We initialize the Blobs protocol in-memory
    let blobs = Blobs::memory().build(&blobs_endpoint);
    // Now we build a router that accepts blobs connections & routes them to the blobs protocol.
    let blobs_router =
        Router::builder(blobs_endpoint).accept(iroh_blobs::ALPN, blobs.clone()).spawn();
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
                let msg = Msg::Bye { at: now() };
                // broadcast the encoded message
                sender.broadcast(msg.to_vec().into()).await?;
                time::sleep(time::Duration::from_millis(100)).await;
                break;
            }
            COMMAND_ME => println!("node_id={node_id}, name={name}"),
            COMMAND_MEMBERS => {
                let members = members.read().await;
                println!("- {node_id}: {name:?}");

                let mut members: Vec<_> = members.iter().collect();
                members.sort_by(|a, b| a.1.cmp(b.1));
                for (node_id, name) in members {
                    println!("- {node_id}: {name:?}")
                }
            }
            COMMAND_COMMAND => {
                let args: Vec<String> = match shell_words::split(&line) {
                    Ok(v) if v.len() > 1 => v[1..].iter().map(|v| v.into()).collect(),
                    _ => {
                        warn!("{command} expected: <args>...\n{EOF_BLOCK}");
                        continue;
                    }
                };

                info!("{command} started: {args:?}");

                let command = command.to_string();
                tokio::task::spawn_blocking(move || {
                    // Command.current_dir("/").env("PATH", "/bin")
                    let output = match Command::new(&args[0]).args(&args[1..]).output() {
                        Ok(v) => v,
                        Err(e) => {
                            error!("{command} error:\n {args:?}: {e}\n{EOF_BLOCK}");
                            return;
                        }
                    };

                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        info!("{command} success: {args:?}\nstdout: {stdout}");
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        error!("{command} failed: {args:?}\nstderr: \n{stderr}");
                    }
                });
            }
            COMMAND_SEND_FILE => {
                // let (filepath, _) = split_first_space(&line[COMMAND_SEND.len()..], true);
                //if filepath.is_empty() {
                //    warn!("no input file\n{EOF_BLOCK}");
                //    continue;
                //};

                let filepath = match shell_words::split(&line) {
                    Ok(args) if args.len() == 2 => args[1].clone(),
                    _ => {
                        warn!("{command} expected: <filepath>\n{EOF_BLOCK}");
                        continue;
                    }
                };

                let filename = filepath.to_string();
                let msg = match read_file_to_send(&filename).await {
                    Ok(content) => Msg::SendFile { filename, content },
                    Err(e) => {
                        error!("{command} error: {filepath}, {e:?}\n{EOF_BLOCK}");
                        continue;
                    }
                };

                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => info!("{command} broadcast ok: {filepath}"),
                    Err(e) => error!("{command} broadcast error: {filepath}, {e:?}"),
                }
            }
            COMMAND_SHARE_FILE => {
                //let (filename, _) = split_first_space(&line[COMMAND_SHARE.len()..], true);
                //if filename.is_empty() {
                //    warn!("no input file\n{EOF_BLOCK}");
                //    continue;
                //};

                let filename = match shell_words::split(&line) {
                    Ok(args) if args.len() == 2 => args[1].clone(),
                    _ => {
                        warn!("{command} expected: <filepath>\n{EOF_BLOCK}");
                        continue;
                    }
                };

                // TODO: async, stop sharing
                let (size, ticket) = match share_file(blobs_client, blobs_node_id, &filename).await
                {
                    Ok(v) => v,
                    Err(e) => {
                        error!("{command} error: {filename}, {e:?}\n{EOF_BLOCK}");
                        continue;
                    }
                };
                info!("{command} blobs: size={size}\n{ticket} {filename}");

                let msg =
                    Msg::ShareFile { filename: filename.to_string(), size, ticket: ticket.clone() };

                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => info!("{command} broadcast ok:\n{ticket} {filename}"),
                    Err(e) => error!("{command} broadcast error: {e:?}"),
                }
            }
            COMMAND_RECEIVE_FILE => {
                //let (ticket, filename) = split_first_space(&line[COMMAND_RECEIVE.len()..], true);

                //let filename = match filename {
                //    Some(v) => v,
                //    None => {
                //        warn!("no filename\n{EOF_BLOCK}");
                //        continue;
                //    }
                //};

                let (ticket, filename) = match shell_words::split(&line) {
                    Ok(args) if args.len() == 3 => (args[1].clone(), args[2].clone()),
                    _ => {
                        warn!("{command} expect: <ticket> <filepath>\n{EOF_BLOCK}");
                        continue;
                    }
                };

                let ticket: BlobTicket = match ticket.parse() {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("{command} invalid ticket: {e:?}\n{EOF_BLOCK}");
                        continue;
                    }
                };

                match receive_file(blobs_client, ticket, filename.to_string()).await {
                    Ok(v) => info!("{command} ok: {filename}, {v}"),
                    Err(e) => error!("{command} error: {filename}, {e:?}"),
                }
            }
            v if v.starts_with(":") => error!("Unknown command: {v:?}"),
            _ => {
                let msg = Msg::Message { text: text };

                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => info!(">>> Message: you({:?})", name),
                    Err(e) => error!(">>> Message: {e:?}"),
                }
            }
        }

        println!("{}", EOF_BLOCK);
    }

    blobs_router.shutdown().await?;
    Ok(())
}
