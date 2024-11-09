use crate::{
    message::{MessageFrames, MessageType},
    Time,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Ping {
    pub client_timestamp: Option<Time>,
    pub server_timestamp: Option<Time>,
}

impl Ping {
    pub fn new() -> Self {
        Self {
            client_timestamp: None,
            server_timestamp: None,
        }
    }
}

impl Default for Ping {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Ping> for MessageFrames {
    fn from(ping: Ping) -> Self {
        let mut frames = MessageFrames::new(MessageType::Ping, 2);

        frames.push_time_or_null(ping.client_timestamp);
        frames.push_time_or_null(ping.server_timestamp);

        frames
    }
}
