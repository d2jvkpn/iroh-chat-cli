use std::{collections::HashMap, fmt::Debug, path, str::FromStr};

use iroh_chat_cli::structs::{Msg, TopicTicket};
use iroh_chat_cli::utils::{self, local_now};
use iroh_chat_cli::{input_loop, subscribe_loop};

use anyhow::Result;
use clap::{ArgAction, Args, Parser};
use iroh::{Endpoint, NodeAddr, RelayMap, RelayMode, RelayUrl, SecretKey, protocol::Router};
/* RelayUrlParseError, RelayNode */
use iroh_gossip::{net::Gossip, proto::TopicId};
use rand::prelude::*;
use tokio::{fs, io::AsyncWriteExt, sync::RwLock};
use tracing::{error, info, warn}; // Level, instrument
use tracing_subscriber::EnvFilter;

const BUILD_INFO: &str = concat!(
    "\nBuildInfo: \n",
    "  build_time: ",
    env!("BUILD_TIME"),
    "\n  version: ",
    env!("CARGO_PKG_VERSION"),
    "\n  git_registry: ",
    env!("GIT_REGISTRY"),
    "\n  git_branch: ",
    env!("GIT_BRANCH"),
    "\n  git_status: ",
    env!("GIT_STATUS"),
    "\n  git_commit_hash: ",
    env!("GIT_COMMIT_HASH"),
    "\n  git_commit_time: ",
    env!("GIT_COMMIT_TIME"),
    "\n"
);

/// Chat over iroh-gossip
///
/// This broadcasts unsigned messages over iroh-gossip.
///
/// By default a new node id is created when starting the example.
///
/// By default, we use the default n0 discovery services to dial by `NodeId`.
#[derive(Parser, Debug)]
#[command(name = "iroh-gossip-cli", version = "1.0", about = "p2p chat inrust from scratch", after_help = BUILD_INFO)]
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

    #[arg(short = 'w', long)]
    write_ticket: Option<String>,

    #[clap(short, long)] // default_value = "configs/local.yaml"
    config: Option<String>,

    #[arg(long)]
    verbose: bool,

    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(Parser, Debug)]
enum Subcommand {
    /// Open a chat room for a topic and print a ticket for others to join.
    // Open,
    Open,

    /// Join a chat room from a ticket.
    Join {
        /// The ticket, as base64 string.
        ticket: String,
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

    let (topic, ticket_nodes) = match &args.subcommand {
        Subcommand::Open => {
            let topic = TopicId::from_bytes(rand::random());
            println!("==> Opening chat room for topic {topic}");
            (topic, vec![])
        }
        Subcommand::Join { ticket } => {
            let TopicTicket { topic, nodes } = TopicTicket::from_str(&ticket)?;
            println!("==> Joining chat room for topic {topic}");
            println!("    nodes_in_ticket: {nodes:?}");
            (topic, nodes)
        }
    };

    let secret_key: SecretKey = match args.config {
        Some(v) => {
            let yaml = utils::load_yaml(&v).unwrap();
            let val = utils::config_get(&yaml, "iroh.secret_key").unwrap();
            let val = serde_yaml::to_string(val)?;
            SecretKey::from_str(&val.trim())?
        }
        None => utils::iroh_secret_key(),
    };

    /*
    let relay_map: RelayMap = args
        .relay_url
        .and_then(|v| Some(v.parse::<RelayUrl>().ok()?))
        .map(RelayNode::from)
        .map(RelayMap::from)
        .unwrap_or_else(|| RelayMap::empty());
    */

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

    let endpoint = Endpoint::builder()
        .relay_mode(RelayMode::Custom(relay_map.clone()))
        .secret_key(secret_key)
        .discovery_n0()
        .bind()
        .await?;

    //let relay_url = endpoint.home_relay().initialized().await.unwrap();
    //println!("==> relay_url: {:?}", relay_url);

    let node_id = endpoint.node_id();
    // Get our address information, includes our `NodeId`, our `RelayUrl`, and any direct addresses.
    let node_addr = endpoint.node_addr().await?;

    // Build and instance of the gossip protocol and add a clone of the endpoint we have built.
    // The gossip protocol will use the endpoint to make connections.
    let gossip = Gossip::builder().spawn(endpoint.clone()).await?;

    // The Router is how we manage protocols on top of the iroh endpoint. It handles all incoming
    // messages and routes them to the correct protocol.
    let router =
        Router::builder(endpoint.clone()).accept(iroh_gossip::ALPN, gossip.clone()).spawn();

    // in our main file, after we create a topic `id`:
    // print a ticket that includes our own node id and endpoint addresses
    let mut all_nodes: Vec<NodeAddr> =
        ticket_nodes.choose_multiple(&mut rand::rng(), 3).map(|x| (*x).clone()).collect();

    all_nodes.push(node_addr.clone());

    let ticket = TopicTicket { topic, nodes: all_nodes };
    // dbg!(&ticket);

    // println!("--> node: {node_addr:?}\n    ticket: {ticket}");
    println!("--> node_id: {}", node_id);
    println!("    relay_url: {:?}", node_addr.relay_url());
    println!("    direct_addresses: {:?}", node_addr.direct_addresses().collect::<Vec<_>>());
    if let Some(v) = args.write_ticket {
        write_topic_ticket(&ticket, &v).await?;
    }
    println!("    ticket: {ticket}");

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

    let msg = Msg::AboutMe { name: name.clone(), at: local_now() };
    sender.broadcast(msg.to_vec().into()).await?;

    let members = std::sync::Arc::new(RwLock::new(HashMap::new()));
    tokio::spawn(subscribe_loop(name.clone(), sender.clone(), receiver, members.clone()));
    // broadcast each line we type
    info!("==> Type a message and hit enter to broadcast...");

    if let Err(e) = input_loop((node_id, name.clone()), sender.clone(), members, relay_map).await {
        error!("input_loop: {e:?}");
    }

    if let Err(e) = router.shutdown().await {
        error!("router.shutdown: {e:?}");
    }

    warn!("<== Quit");
    Ok(())
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
    println!("    saved_ticket: {}", filepath.display());

    Ok(())
}
