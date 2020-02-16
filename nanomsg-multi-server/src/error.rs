use bincode::ErrorKind as BincodeError;
use nanomsg::result::Error as NanoError;
use std::error::Error;
use std::io::Error as IoError;

#[derive(Debug)]
pub struct SerializationError(BincodeError);

impl From<Box<BincodeError>> for SerializationError {
    #[cfg_attr(feature = "cargo-clippy", allow(boxed_local))]
    fn from(source: Box<BincodeError>) -> SerializationError {
        SerializationError(*source)
    }
}

#[derive(Debug)]
pub enum ServerError {
    SocketError(NanoError),
    Serialization(SerializationError),
    PeerConnection,
    BufferOverflow,
    IoError(IoError),
    Unknown,
}

impl From<NanoError> for ServerError {
    fn from(source: NanoError) -> ServerError {
        ServerError::SocketError(source)
    }
}

impl From<PeerError> for ServerError {
    fn from(_source: PeerError) -> ServerError {
        ServerError::PeerConnection
    }
}

impl From<SerializationError> for ServerError {
    fn from(source: SerializationError) -> ServerError {
        ServerError::Serialization(source)
    }
}

impl From<IoError> for ServerError {
    fn from(source: IoError) -> ServerError {
        ServerError::IoError(source)
    }
}

impl From<()> for ServerError {
    fn from(_source: ()) -> ServerError {
        ServerError::Unknown
    }
}

#[derive(Debug)]
pub enum PeerError {
    Socket(NanoError),
    Io(IoError),
    BufferOverflow,
    Serialization(SerializationError),
    BadMessage,
    Unknown,
}

impl From<NanoError> for PeerError {
    fn from(source: NanoError) -> PeerError {
        PeerError::Socket(source)
    }
}

impl From<SerializationError> for PeerError {
    fn from(source: SerializationError) -> PeerError {
        PeerError::Serialization(source)
    }
}

impl From<IoError> for PeerError {
    fn from(source: IoError) -> PeerError {
        PeerError::Io(source)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ControlReplyError {
    Socket(String),
    PeerConnection,
    Internal,
}

impl From<ServerError> for ControlReplyError {
    fn from(source: ServerError) -> ControlReplyError {
        use crate::ServerError::*;

        match source {
            SocketError(nano_error) => {
                ControlReplyError::Socket(nano_error.description().to_owned())
            }
            Serialization(_) | PeerConnection => ControlReplyError::PeerConnection,
            Unknown | BufferOverflow | IoError(_) => ControlReplyError::Internal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum PeerReplyError {
    Unknown,
    Socket,
    BufferOverflow,
    Serialization,
    BadMessage,
    Io,
}

impl From<PeerError> for PeerReplyError {
    fn from(source: PeerError) -> PeerReplyError {
        use crate::PeerError::*;

        match source {
            Unknown => PeerReplyError::Unknown,
            Socket(_) => PeerReplyError::Socket,
            BufferOverflow => PeerReplyError::BufferOverflow,
            Serialization(_) => PeerReplyError::Serialization,
            BadMessage => PeerReplyError::BadMessage,
            Io(_) => PeerReplyError::Io,
        }
    }
}
