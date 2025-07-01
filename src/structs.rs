use std::{collections::HashMap, fmt, str::FromStr};

// use crate::utils::local_now;

use anyhow::{Result, anyhow};
use ed25519::Signature;
// use base64::{Engine, engine::general_purpose};
use bytes::Bytes;
use iroh::{NodeAddr, NodeId, SecretKey};
use iroh_blobs::ticket::BlobTicket;
use iroh_gossip::proto::TopicId;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

pub const COMMAND_QUIT: &str = "::quit";
pub const COMMAND_ME: &str = "::me";
pub const COMMAND_MEMBERS: &str = "::members";
pub const COMMAND_RUN: &str = "::run";

pub const COMMAND_SEND_FILE: &str = "::send_file";
pub const COMMAND_SHARE_FILE: &str = "::share_file";
pub const COMMAND_RECEIVE_FILE: &str = "::receive_file";

pub const MAX_FILESIZE: u64 = 8 * 1024 * 1024;
pub const EOF_BLOCK: &str = "----------------------------------------------------------------";

// add the message code to the bottom
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub from: NodeId,
    nonce: [u8; 16],
    pub at: i64,
    pub msg: Msg,
}

impl Message {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }

    pub fn new(node_id: NodeId, msg: Msg) -> Self {
        Self {
            from: node_id,
            nonce: rand::random(),
            at: chrono::Utc::now().timestamp_millis(),
            msg,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde_json::to_vec is infallible")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Msg {
    AboutMe { name: String },
    Bye,
    Message { text: String },
    SendFile { filename: String, content: Vec<u8> },
    ShareFile { filename: String, size: u64, ticket: BlobTicket },
}

/*
impl Msg {
    pub fn to_vec(self, from: NodeId) -> Vec<u8> {
        serde_json::to_vec(&Message {
            nonce: rand::random(),
            at: chrono::Utc::now().timestamp_millis(),
            from,
            msg: self,
        })
        .expect("serde_json::to_vec is infallible")
    }
}
*/

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicTicket {
    pub topic: TopicId,
    pub nodes: Vec<NodeAddr>,
}

impl TopicTicket {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde_json::to_vec is infallible")
    }

    /*
    pub fn base64_bytes(&self) -> Vec<u8> {
        let bts = serde_json::to_vec(self).expect("serde_json::to_vec is infallible");
        general_purpose::STANDARD.encode(bts).into()
    }
    */

    pub fn base32_bytes(&self) -> Vec<u8> {
        let bts = serde_json::to_vec(self).expect("serde_json::to_vec is infallible");
        let mut text = data_encoding::BASE32_NOPAD.encode(&bts);
        text.make_ascii_lowercase();
        text.into()
    }
}

impl fmt::Display for TopicTicket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // let text = general_purpose::STANDARD.encode(&self.to_bytes()[..]);
        let mut text = data_encoding::BASE32_NOPAD.encode(&self.to_bytes()[..]);
        text.make_ascii_lowercase();
        write!(f, "{}", text)
    }
}

impl FromStr for TopicTicket {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // let bytes = general_purpose::STANDARD.decode(s.as_bytes())?;
        let bytes = data_encoding::BASE32_NOPAD.decode(s.to_ascii_uppercase().as_bytes())?;
        Self::from_bytes(&bytes)
    }
}

pub fn parse_raw_message(bts: &Bytes) -> Result<Message> {
    if bts.len() <= 64 {
        return Err(anyhow!("invalid length: {}", bts.len()));
    }

    let signature =
        Signature::from_slice(&bts[..64]).map_err(|e| anyhow!("parse signature: {e:?}"))?;

    let message = Message::from_bytes(&bts[64..]).map_err(|e| anyhow!("parse message: {e:?}"))?;

    message.from.verify(&bts[64..], &signature).map_err(|e| anyhow!("verify signature: {e:?}"))?;

    // TODO: nonce, at

    Ok(message)
}

#[derive(Clone)]
pub struct MemDB {
    secret_key: SecretKey,
    node_id: NodeId,
    name: String,
    pub members: std::sync::Arc<RwLock<HashMap<NodeId, String>>>,
}

impl MemDB {
    pub fn new(secret_key: SecretKey, node_id: NodeId, name: String) -> Self {
        Self {
            secret_key,
            node_id,
            name,
            members: std::sync::Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn node(&self) -> (NodeId, String) {
        (self.node_id, self.name.clone())
    }

    pub fn sign_msg(&self, msg: Msg) -> Bytes {
        let bts = serde_json::to_vec(&Message {
            from: self.node_id,
            nonce: rand::random(),
            at: chrono::Utc::now().timestamp_millis(),
            msg,
        })
        .expect("serde_json::to_vec is infallible");

        let signature = self.secret_key.sign(&bts);

        let mut buf = Vec::with_capacity(64 + bts.len());

        buf.extend(&signature.to_bytes());
        buf.extend(bts);

        buf.into()
    }

    pub fn sign_message(&self, message: &Message) -> Bytes {
        let bts = serde_json::to_vec(message).expect("serde_json::to_vec is infallible");
        let signature = self.secret_key.sign(&bts);

        let mut buf = Vec::with_capacity(64 + bts.len());

        buf.extend(&signature.to_bytes());
        buf.extend(bts);

        buf.into()
    }
}

impl fmt::Display for MemDB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "node_id={}, name={}", self.node_id, self.name)
    }
}
