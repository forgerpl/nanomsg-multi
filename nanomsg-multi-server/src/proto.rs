use serde::{Deserialize, Serialize};

pub use error::{ControlReplyError, PeerError, PeerReplyError, SerializationError};

pub fn serialize<T>(value: &T) -> Result<Vec<u8>, SerializationError>
where
    T: ?Sized + Serialize,
{
    use bincode::{serialize, Infinite};

    Ok(serialize(value, Infinite)?)
}

pub fn deserialize<'de, T>(bytes: &'de [u8]) -> Result<T, SerializationError>
where
    T: Deserialize<'de>,
{
    use bincode::deserialize;

    Ok(deserialize(bytes)?)
}

/// Id of client connection
pub type ConnId = usize;

/// Id of a specific message
pub type MessageId = usize;

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlRequest {
    /// A client wants to connect
    CreateSocket,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlReply {
    /// A dedicated Pair socket was created for this client
    SocketCreated(Result<(ConnId, String), ControlReplyError>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PeerRequest {
    /// A Request schedule message
    Request(MessageId, Option<Vec<u8>>),
    /// Abort processing previously scheduled message
    Abort(MessageId),
    /// Connection will be closed
    CloseConnection,
    /// KeepAlive ping
    KeepAlive,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PeerReply {
    /// A reponse to a previously scheduled Request
    Response(MessageId, Result<Option<Vec<u8>>, PeerReplyError>),
    /// KeepAlive ping
    KeepAlive,
}
