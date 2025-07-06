use std::{fmt::Debug, path, str::FromStr};

use iroh_chat_cli::structs::{MemDB, Msg, TopicTicket};
use iroh_chat_cli::utils::{self, build_info};
use iroh_chat_cli::{input_loop, subscribe_loop};

use anyhow::{Result, anyhow};
use clap::{ArgAction, Args, Parser};
use futures::{FutureExt, pin_mut};
use iroh::{Endpoint, NodeAddr, RelayMap, RelayMode, RelayUrl, SecretKey, protocol::Router};
use tokio_util::sync::CancellationToken;
/* RelayUrlParseError, RelayNode */
use iroh_gossip::{net::Gossip, proto::TopicId};
use rand::prelude::*;
use tokio::{fs, io::AsyncWriteExt, signal};
use tracing::{error, info, warn}; // Level, instrument
use tracing_subscriber::EnvFilter;

/// Chat over iroh-gossip
///
/// This broadcasts unsigned messages over iroh-gossip.
///
/// By default a new node id is created when starting the example.
///
/// By default, we use the default n0 discovery services to dial by `NodeId`.
#[derive(Parser, Debug)]
#[command(
    name = "iroh-chat-cli",
    version = "1.0",
    about = "p2p chat inrust from scratch",
    after_help = build_info(),
)]
struct Command {
    /*
    /// Set the bind port for our socket. By default, a random port will be used.
    #[clap(short, long, default_value = "0")]
    bind_port: u16,
    */
    /// Set your nickname.
    #[clap(short, long)]
    name: String,

    #[arg(short = 'r', long, action=ArgAction::Append)]
    relay_url: Vec<String>,

    #[clap(short, long)] // default_value = "configs/local.yaml"
    config: Option<String>,

    /// run in debug mode
    #[arg(long)]
    verbose: bool,

    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(Parser, Debug)]
enum Subcommand {
    /// Open a chat room for a topic and print a ticket for others to join.
    Open {
        /// Optional file path to save the ticket; by default, the ticket is printed.
        #[arg(short = 'w', long)]
        write_ticket: Option<String>,
    },

    /// Join a chat room from a ticket.
    Join {
        /// The ticket can be provided as a base32 string or a file path.
        ticket: String,

        /// Optional file path to save the ticket; by default, the ticket is printed.
        #[arg(short = 'w', long)]
        write_ticket: Option<String>,
    },
    // Join(JoinCommand),
}

#[derive(Debug, Args)]
struct JoinCommand {
    ticket: String,

    // --ticket t1 --ticket t2 --ticket t3
    #[arg(short='t', long ="ticket", action=ArgAction::Append)]
    tickets_v1: Vec<String>,

    /// t1 t2 t3
    #[arg(required = true, num_args = 1..)]
    tickets_v2: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Command::parse();
    let name = args.name.clone();

    let filter = if args.verbose {
        EnvFilter::new("debug")
    } else {
        // EnvFilter::new(format!("{0}=info,{0}::handlers=info", module_path!()))
        EnvFilter::new(format!("{0}=info", module_path!()))
    };
    utils::log2stdout(filter);

    let (topic, ticket_nodes, write_ticket) = match &args.subcommand {
        Subcommand::Open { write_ticket } => {
            let topic = TopicId::from_bytes(rand::random());
            println!("==> Opening chat room for topic {topic}");
            (topic, vec![], write_ticket)
        }
        Subcommand::Join { ticket: ticket_str, write_ticket } => {
            // let TopicTicket { topic, nodes } = if ticket.contains(".") {
            let topic_ticket = if ticket_str.contains(".") {
                let ticket_str = fs::read_to_string(&ticket_str).await?;
                TopicTicket::from_str(&ticket_str.trim())?
            } else {
                TopicTicket::from_str(&ticket_str)?
            };

            println!("==> Joining chat room for ticket: {topic_ticket:?}");
            (topic_ticket.topic, topic_ticket.nodes, write_ticket)
        }
    };

    //let relay_url = endpoint.home_relay().initialized().await.unwrap();
    //println!("==> relay_url: {:?}", relay_url);
    let relay_map = if args.relay_url.is_empty() {
        iroh::defaults::prod::default_relay_map()
    } else if args.relay_url.len() == 1 && args.relay_url[0] == "none" {
        RelayMap::empty()
    } else {
        let mut urls = Vec::with_capacity(args.relay_url.len());

        for v in args.relay_url {
            let v = v.parse::<RelayUrl>()?;
            urls.push(v);
        }

        RelayMap::from_iter(urls)
    };

    let secret_key: SecretKey = match args.config {
        Some(v) => {
            let yaml = utils::load_yaml(&v)?;

            let val = utils::config_get(&yaml, "iroh.secret_key")
                .ok_or(anyhow!("can't get iroh.secret_key from config"))?;

            let val = serde_yaml::to_string(val)?;
            SecretKey::from_str(&val.trim())?
        }
        None => utils::iroh_secret_key(),
    };

    let endpoint = Endpoint::builder()
        .relay_mode(RelayMode::Custom(relay_map.clone()))
        .secret_key(secret_key.clone())
        .discovery_n0()
        .bind()
        .await?;

    let mem_db = MemDB::new(secret_key, endpoint.node_id(), name.clone());
    // Get our address information, includes our `NodeId`, our `RelayUrl`, and any direct addresses.
    let node_addr = endpoint.node_addr().await?;

    // Build and instance of the gossip protocol and add a clone of the endpoint we have built.
    // The gossip protocol will use the endpoint to make connections.
    let gossip = Gossip::builder().spawn(endpoint.clone()).await?;

    // The Router is how we manage protocols on top of the iroh endpoint. It handles all incoming
    // messages and routes them to the correct protocol.
    let router =
        Router::builder(endpoint.clone()).accept(iroh_gossip::ALPN, gossip.clone()).spawn();
    // println!("iroh_gossip::ALPN: {}", String::from_utf8(iroh_gossip::ALPN.to_vec()).unwrap());
    // iroh_gossip::ALPN: /iroh-gossip/0

    // in our main file, after we create a topic `id`:
    // print a ticket that includes our own node id and endpoint addresses
    let mut all_nodes: Vec<NodeAddr> =
        ticket_nodes.choose_multiple(&mut rand::rng(), 3).map(|x| (*x).clone()).collect();

    all_nodes.push(node_addr.clone());

    let ticket = TopicTicket { topic, nodes: all_nodes };
    // dbg!(&ticket);

    // println!("--> node: {node_addr:?}\n    ticket: {ticket}");
    println!("--> node: {:?}", mem_db.node());
    println!("    relay_url: {:?}", node_addr.relay_url());
    println!("    direct_addresses: {:?}", node_addr.direct_addresses().collect::<Vec<_>>());
    if let Some(v) = write_ticket {
        write_topic_ticket(&ticket, &v).await?;
        println!("    ticket: {v}");
    } else {
        println!("    ticket: {ticket}");
    }

    // join the gossip topic by connecting to known nodes, if any
    let node_ids = ticket_nodes.iter().map(|p| p.node_id).collect();

    if ticket_nodes.is_empty() {
        info!("waiting for nodes to join us...");
    } else {
        // add the peer addrs from the ticket to our endpoint's addressbook,
        // so that they can be dialed
        for node in ticket_nodes.into_iter() {
            // println!("--> trying to connect to node: {:?}...", node);
            if let Err(e) = endpoint.add_node_addr(node.clone()) {
                warn!("can't connect to node: {}, {e:?}", node.node_id);
            } else {
                info!("connected to node: {}", node.node_id);
            }
        }
    }

    // dbg!(&node_ids);
    let (sender, receiver) = gossip.subscribe_and_join(topic, node_ids).await?.split();
    info!("connected!");

    let about_me = Msg::AboutMe { name: name.clone() };
    sender.broadcast(mem_db.sign_msg(about_me).into()).await?;

    /*
    tokio::spawn(subscribe_loop(mem_db.clone(), sender.clone(), receiver));

    if let Err(e) = input_loop(mem_db.clone(), sender.clone(), relay_map).await {
        error!("input_loop: {e:?}");
    }
    */

    let cancel_token = CancellationToken::new();

    let task1 = tokio::task::spawn(subscribe_loop(
        cancel_token.clone(),
        mem_db.clone(),
        sender.clone(),
        receiver,
    ));

    let task2 = tokio::task::spawn(input_loop(
        cancel_token.clone(),
        mem_db.clone(),
        sender.clone(),
        relay_map,
    ));

    let (fuse1, fuse2) = (task1.fuse(), task2.fuse());
    pin_mut!(fuse1, fuse2);

    tokio::select! {
        _ = &mut fuse1 => {
            warn!("subscribe_loop exited.");
        }
        _ = &mut fuse2 => warn!("input_loop exited."),
        _ = signal::ctrl_c() => {
            println!("");
            error!("<-- received Ctrl+C.");
        }
    }
    warn!("--> cancel token");
    cancel_token.cancel();

    let (result1, result2) = tokio::join!(fuse1, fuse2);
    if let Err(e) = result1 {
        error!("subscribe_loop: {e}");
    }
    if let Err(e) = result2 {
        error!("input_loop: {e}");
    }

    router.shutdown().await?;

    warn!("<== Quit");
    std::process::exit(0);
}

pub async fn write_topic_ticket(ticket: &TopicTicket, filename: &str) -> Result<()> {
    // fs::create_dir_all(dir).await?;
    // let filepath = dir.join(format!("{}.topic.ticket", filename));

    let filepath = path::Path::new(filename);
    if let Some(p) = filepath.parent() {
        fs::create_dir_all(p).await?;
    }

    let mut file = fs::File::create(&filepath).await?;
    // file.write_all(&ticket.to_bytes()).await?;
    file.write_all(&ticket.base32_bytes()).await?;
    file.write_all(b"\n").await?;

    Ok(())
}
