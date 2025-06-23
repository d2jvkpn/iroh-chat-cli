use std::{collections::HashMap, process::Command};

use crate::structs::{
    COMMAND_COMMAND, COMMAND_ME, COMMAND_MEMBERS, COMMAND_QUIT, COMMAND_RECEIVE_FILE,
    COMMAND_SEND_FILE, COMMAND_SHARE_FILE, EOF_EVENT, EOF_MESSAGE, Msg,
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
                let msg = Msg::Bye { at: now() };
                // broadcast the encoded message
                sender.broadcast(msg.to_vec().into()).await?;
                time::sleep(time::Duration::from_millis(100)).await;
                break;
            }
            COMMAND_ME => println!("ME: {node_id}, {name}\n{EOF_EVENT}"),
            COMMAND_MEMBERS => {
                let members = members.read().await;

                println!("members:");
                println!("  {node_id}: name");
                let mut members: Vec<_> = members.iter().collect();
                members.sort_by(|a, b| a.1.cmp(b.1));
                for (node_id, name) in members {
                    println!("  {node_id}: {name:?}")
                }

                info!("{EOF_EVENT}");
            }
            COMMAND_COMMAND => {
                let args: Vec<String> = match shell_words::split(&line) {
                    Ok(v) if v.len() > 1 => v[1..].iter().map(|v| v.into()).collect(),
                    _ => {
                        warn!("{COMMAND_COMMAND} expected: <args>...\n{EOF_EVENT}");
                        continue;
                    }
                };

                info!("--> {COMMAND_COMMAND} started: {args:?}\n{EOF_EVENT}");
                tokio::task::spawn_blocking(move || {
                    // Command.current_dir("/").env("PATH", "/bin")
                    let output = match Command::new(&args[0]).args(&args[1..]).output() {
                        Ok(v) => v,
                        Err(e) => {
                            error!("{COMMAND_COMMAND} error: {e}\n{EOF_EVENT}");
                            return;
                        }
                    };

                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        info!("{COMMAND_COMMAND} stdout: {args:?}\n{stdout}\n{EOF_EVENT}");
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        error!("{COMMAND_COMMAND} stderr: {args:?}\n{stderr}\n{EOF_EVENT}");
                    }
                });
            }
            COMMAND_SEND_FILE => {
                // let (filepath, _) = split_first_space(&line[COMMAND_SEND.len()..], true);
                //if filepath.is_empty() {
                //    warn!("no input file\n{EOF_EVENT}");
                //    continue;
                //};

                let filepath = match shell_words::split(&line) {
                    Ok(args) if args.len() == 2 => args[1].clone(),
                    _ => {
                        warn!("{COMMAND_SEND_FILE} expected: <filepath>\n{EOF_EVENT}");
                        continue;
                    }
                };

                let filename = filepath.to_string();
                let msg = match read_file_to_send(&filename).await {
                    Ok(content) => Msg::SendFile { filename, content },
                    Err(e) => {
                        error!("{COMMAND_SEND_FILE} error: {filepath}, {e:?}\n{EOF_EVENT}");
                        continue;
                    }
                };

                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => info!("{COMMAND_SEND_FILE}: {filepath}\n{EOF_EVENT}"),
                    Err(e) => error!("{COMMAND_SEND_FILE} error: {filepath}, {e:?}\n{EOF_EVENT}"),
                }
            }
            COMMAND_SHARE_FILE => {
                //let (filename, _) = split_first_space(&line[COMMAND_SHARE.len()..], true);
                //if filename.is_empty() {
                //    warn!("no input file\n{EOF_EVENT}");
                //    continue;
                //};

                let filename = match shell_words::split(&line) {
                    Ok(args) if args.len() == 2 => args[1].clone(),
                    _ => {
                        warn!("{COMMAND_SHARE_FILE} expected: <filepath>\n{EOF_EVENT}");
                        continue;
                    }
                };

                // TODO: async, stop sharing
                let (size, ticket) = match share_file(blobs_client, node_id, &filename).await {
                    Ok(v) => v,
                    Err(e) => {
                        error!("{COMMAND_SHARE_FILE}: {filename}, {e:?}\n{EOF_EVENT}");
                        continue;
                    }
                };
                info!("{COMMAND_SHARE_FILE} blobs: size={size}\n{ticket} {filename}\n{EOF_EVENT}");

                let msg = Msg::ShareFile { filename: filename.to_string(), size, ticket };

                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => info!("{COMMAND_SHARE_FILE} broadcast ok\n{EOF_MESSAGE}"),
                    Err(e) => error!("{COMMAND_SHARE_FILE} broadcast error: {e:?}\n{EOF_EVENT}"),
                }
            }
            COMMAND_RECEIVE_FILE => {
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
                        warn!("{COMMAND_RECEIVE_FILE} expect: <ticket> <filepath>\n{EOF_EVENT}");
                        continue;
                    }
                };

                let ticket: BlobTicket = match ticket.parse() {
                    Ok(v) => v,
                    Err(e) => {
                        warn!("{COMMAND_RECEIVE_FILE} invalid ticket: {e:?}\n{EOF_EVENT}");
                        continue;
                    }
                };

                match receive_file(blobs_client, ticket, filename.to_string()).await {
                    Ok(v) => info!("{COMMAND_RECEIVE_FILE} ok: {filename}\n{v}\n{EOF_EVENT}"),
                    Err(e) => {
                        error!("{COMMAND_RECEIVE_FILE} error: {filename}, {e:?}\n{EOF_EVENT}")
                    }
                }
            }
            _ => {
                let msg = Msg::Message { text: text };

                match sender.broadcast(msg.to_vec().into()).await {
                    Ok(_) => info!(">>> You({:?})\n{EOF_MESSAGE}", name),
                    Err(e) => error!("BroadcastMsg error: {e:?}\n{EOF_MESSAGE}"),
                }
            }
        }
    }

    Ok(())
}
