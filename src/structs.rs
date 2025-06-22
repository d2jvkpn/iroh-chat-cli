use std::{fmt, str::FromStr};

use anyhow::Result;
// use base64::{Engine, engine::general_purpose};
use iroh::{NodeAddr, NodeId};
use iroh_blobs::ticket::BlobTicket;
use iroh_gossip::proto::TopicId;
use serde::{Deserialize, Serialize};

pub const COMMAND_QUIT: &str = ":quit";
pub const COMMAND_ME: &str = ":me";
pub const COMMAND_ONLINE: &str = ":online";

pub const COMMAND_SEND: &str = ":send";
pub const COMMAND_SHARE: &str = ":share";
pub const COMMAND_RECEIVE: &str = ":receive";

pub const MAX_FILESIZE: u64 = 8 * 1024 * 1024;

pub const EOF_MESSAGE: &str = "----------------------------------------------------------------";
pub const EOF_EVENT: &str = "++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++++";

#[derive(Debug, Serialize, Deserialize)]
pub enum Msg {
    AboutMe { from: NodeId, name: String, at: String },
    Message { from: NodeId, text: String },
    File { from: NodeId, filename: String, content: Vec<u8> },
    Share { from: NodeId, filename: String, size: u64, ticket: BlobTicket },
    Bye { from: NodeId, at: String },
}

impl Msg {
    pub fn to_vec(self) -> Vec<u8> {
        serde_json::to_vec(&Message { msg: self, nonce: rand::random() })
            .expect("serde_json::to_vec is infallible")
    }
}

// add the message code to the bottom
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub msg: Msg,
    nonce: [u8; 16],
}

impl Message {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }

    pub fn new(msg: Msg) -> Self {
        Self { msg, nonce: rand::random() }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde_json::to_vec is infallible")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TopicTicket {
    pub topic: TopicId,
    pub nodes: Vec<NodeAddr>,
}

impl TopicTicket {
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
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
