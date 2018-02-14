# nanomsg multisocket wrapper

## About

### What this library does?

### What problem does it solve?

## Using it

```rust
extern crate colored_logger;
extern crate flexi_logger;
extern crate futures;
#[macro_use]
extern crate log;
extern crate nanomsg_multi_server;
extern crate tokio_core;

use tokio_core::reactor::Core;
use nanomsg_multi_server::MultiServer;
use nanomsg_multi_server::proto::{PeerReply, PeerRequest};
use nanomsg_multi_server::config::{GcInterval, MainSocketUrl, SessionTimeout};

use futures::{Future, Sink, Stream};

fn main() {
    flexi_logger::Logger::with_env()
        .format(colored_logger::formatter)
        .start()
        .expect("Logger initialization failed");

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    info!("Starting socket server");

    let server = MultiServer::new(
        MainSocketUrl::default(),
        SessionTimeout::default(),
        GcInterval::default(),
        handle.clone(),
    );

    let (server, sessions) = server.into_futures().expect("server error");

    handle.spawn(sessions.for_each(|session| {
        info!("Incoming new session {:?}", session);

        let (writer, reader) = session.connection.split();

        handle.spawn(reader
            .map(move |msg| {
                use self::PeerRequest::*;

                match msg {
                    Request(msgid, Some(msg)) => {
                        info!(
                            "[{connid}@{msgid}] Incoming Request",
                            connid=connid, msgid=msgid
                        );

                        let reply = process_message(msg);

                        Ok(PeerReply::Response(msgid, Ok(Some(reply))))
                    }
                    Abort(msgid) => {
                        info!(
                            "[{connid}@{msgid}] Abort Request",
                            connid=connid, msgid=msgid
                        );

                        Ok(PeerReply::Response(msgid, Ok(None)))
                    }
                    _ => Err(PeerError::BadMessage),
                }
            })
            .and_then(|reply| reply)
            .forward(writer)
            .map(|_| ())
            .map_err(|error| error!("Session connection error {:?}", error)));
    }));

    core.run(server);
}

```

## Specification

```rust
/// Id of client connection
pub type ConnId = usize;

/// Id of a specific message
pub type MessageId = usize;

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum ControlRequest {
    /// A client wants to connect
    CreateSocket,
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ControlReplyError {
    Socket(String),
    PeerConnection,
    Internal,
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
    /// Keepalive ping
    KeepAlive,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PeerReply {
    /// A reponse to a previously scheduled Request
    Response(MessageId, Result<Option<Vec<u8>>, PeerReplyError>),
    /// Keepalive ping
    KeepAlive,
}
```
