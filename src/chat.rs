use std::{collections::HashMap, fmt::Debug, str::FromStr, sync::Arc};

use iroh_chat_cli::handlers::{input_loop, subscribe_loop};
use iroh_chat_cli::structs::{Msg, Ticket};
use iroh_chat_cli::utils::{self, now};

use anyhow::Result;
use clap::{ArgAction, Args, Parser};
use iroh::protocol::Router;
use iroh::{Endpoint, NodeAddr, RelayMap, RelayMode, RelayNode, RelayUrl, SecretKey};
use iroh_gossip::{ALPN, net::Gossip, proto::TopicId};
use rand::prelude::*;
use tokio::sync::RwLock;

/// Chat over iroh-gossip
///
/// This broadcasts unsigned messages over iroh-gossip.
///
/// By default a new node id is created when starting the example.
///
/// By default, we use the default n0 discovery services to dial by `NodeId`.
#[derive(Parser, Debug)]
#[command(name = "iroh-gossip-cli", version = "1.0", about = "p2p chat inrust from scratch")]
struct Command {
    #[clap(subcommand)]
    subcommand: Subcommand,

    #[clap(short, long)] // default_value = "configs/local.yaml"
    config: Option<String>,

    #[clap(short, long)]
    relay_url: Option<String>,

    /*
    /// Set the bind port for our socket. By default, a random port will be used.
    #[clap(short, long, default_value = "0")]
    bind_port: u16,
    */
    /// Set your nickname.
    #[clap(short, long)]
    name: String,
}

#[derive(Parser, Debug)]
enum Subcommand {
    /// Open a chat room for a topic and print a ticket for others to join.
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
    #[arg(short = 't', long = "ticket", action = ArgAction::Append)]
    tickets_v1: Vec<String>,

    /// t1 t2 t3
    #[arg(required = true, num_args = 1..)]
    tickets_v2: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Command::parse();
    let name = args.name.clone();

    let (topic, ticket_nodes) = match &args.subcommand {
        Subcommand::Open => {
            let topic = TopicId::from_bytes(rand::random());
            println!("==> Opening chat room for topic {topic}");
            (topic, vec![])
        }
        Subcommand::Join { ticket } => {
            let Ticket { topic, nodes } = Ticket::from_str(&ticket)?;
            println!("==> Joining chat room for topic {topic}");
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

    let relay_map: RelayMap = args
        .relay_url
        .and_then(|v| Some(v.parse::<RelayUrl>().ok()?))
        .map(RelayNode::from)
        .map(RelayMap::from)
        .unwrap_or_else(|| RelayMap::empty());

    let endpoint = if relay_map.is_empty() {
        Endpoint::builder() // use default relay url: https://euw1-1.relay.iroh.network
    } else {
        Endpoint::builder().relay_mode(RelayMode::Custom(relay_map))
    }
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
    let router = Router::builder(endpoint.clone()).accept(ALPN, gossip.clone()).spawn();

    // in our main file, after we create a topic `id`:
    // print a ticket that includes our own node id and endpoint addresses
    let mut all_nodes: Vec<NodeAddr> =
        ticket_nodes.choose_multiple(&mut rand::rng(), 2).map(|x| (*x).clone()).collect();

    all_nodes.push(node_addr);

    let ticket = Ticket { topic, nodes: all_nodes };
    utils::write_ticket(&ticket, &name).await?;

    // join the gossip topic by connecting to known nodes, if any
    let node_ids = ticket_nodes.iter().map(|p| p.node_id).collect();

    if ticket_nodes.is_empty() {
        println!("--> {} waiting for nodes to join us...", now());
    } else {
        // add the peer addrs from the ticket to our endpoint's addressbook,
        // so that they can be dialed
        for node in ticket_nodes.into_iter() {
            // println!("--> trying to connect to node: {:?}...", node);
            if let Err(e) = endpoint.add_node_addr(node.clone()) {
                println!("!!! {} can't connect to node: {}, {e:?}", now(), node.node_id);
            } else {
                println!("--> {} connected to node: {}", now(), node.node_id);
            }
        }
    }

    dbg!(&node_ids);
    let (sender, receiver) = gossip.subscribe_and_join(topic, node_ids).await?.split();
    println!("--> {} connected!", now());

    let msg = Msg::AboutMe { from: node_id, name: name.clone(), at: now() };
    sender.broadcast(msg.to_vec().into()).await?;

    let members = Arc::new(RwLock::new(HashMap::new()));
    tokio::spawn(subscribe_loop(node_id, name.clone(), sender.clone(), receiver, members.clone()));
    // broadcast each line we type
    println!("==> Type a message and hit enter to broadcast...");
    input_loop(node_id, name.clone(), sender.clone(), members).await?;

    println!("<== {} Quit", now());
    router.shutdown().await?;
    Ok(())
}
