use std::{fmt, str::FromStr};

use anyhow::Result;
use base64::{Engine, engine::general_purpose};
use iroh::{NodeAddr, NodeId};
use iroh_gossip::proto::TopicId;
use serde::{Deserialize, Serialize};

pub const COMMAND_QUIT: &str = ":quit";

pub const EOF_MESSAGE: &str = "--------------------------------";
pub const EOF_EVENT: &str = "++++++++++++++++++++++++++++++++";
pub const EOF_ERROR: &str = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";

// add the message code to the bottom
#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub body: MessageBody,
    nonce: [u8; 16],
}

#[derive(Debug, Serialize, Deserialize)]
pub enum MessageBody {
    AboutMe { from: NodeId, name: String, at: String },
    Message { from: NodeId, text: String },
    Bye { from: NodeId, at: String },
}

impl Message {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }

    pub fn new(body: MessageBody) -> Self {
        Self { body, nonce: rand::random() }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde_json::to_vec is infallible")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ticket {
    pub topic: TopicId,
    pub nodes: Vec<NodeAddr>,
}

impl Ticket {
    fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(Into::into)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("serde_json::to_vec is infallible")
    }
}

impl fmt::Display for Ticket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = general_purpose::STANDARD.encode(&self.to_bytes()[..]);
        write!(f, "{}", text)
    }
}

impl FromStr for Ticket {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = general_purpose::STANDARD.decode(s.as_bytes())?;
        Self::from_bytes(&bytes)
    }
}
