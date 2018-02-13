#[macro_use]
extern crate log;

extern crate futures;
extern crate nanomsg;
extern crate nanomsg_tokio;
extern crate rand;
extern crate tokio_core;

mod error;
#[macro_use]
mod consts;
mod config;

use error::ServerError;
use config::{GcInterval, MainSocketUrl, SessionTimeout};

use futures::{Async, AsyncSink, Future, Poll, Sink, StartSend, Stream};
use futures::stream::{SplitSink, SplitStream};
use futures::unsync::mpsc::unbounded;
use futures::unsync::oneshot::{channel, Sender as OneshotSender};

use nanomsg_tokio::Socket as NanoSocket;
use nanomsg::Protocol;
use nanomsg::result::Error as NanoError;

use tokio_core::reactor::{Core, Handle, Interval, Timeout};

use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;
use std::str::from_utf8;
use std::time::{Duration, Instant};

/// Id of client connection
pub type ConnId = usize;

/// Id of a specific message
pub type MessageId = usize;

#[derive(Debug)]
struct ConnData {
    last_active: Instant,
    killswitch: Option<OneshotSender<()>>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ControlMessage {
    /// A client wants to connect
    ConnectionRequest,
    /// A dedicated PAIR socket was created for the client
    PairOpened(ConnId),
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PeerError {
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PeerMessage {
    Request(MessageId, Option<Vec<u8>>),
    Response(MessageId, Result<Option<Vec<u8>>, PeerError>),
    /// Connection will be closed
    ConnectionClose,
    /// Keepalive ping
    Ping,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Request {
        connid: ConnId,
        msgid: MessageId,
        data: Option<Vec<u8>>,
    },

    Response {
        connid: ConnId,
        msgid: MessageId,
        data: Result<Option<Vec<u8>>, PeerError>,
    },
}

#[derive(Debug)]
pub struct MultiServer {
    session_timeout: Duration,
    gc_interval: Duration,
    reader: SplitStream<NanoSocket>,
    writer: SplitSink<NanoSocket>,
    handle: Handle,
    connections: Rc<RefCell<HashMap<ConnId, ConnData>>>,
}

impl MultiServer {
    pub fn new<'s, MS, ST, GI>(
        handle: &Handle,
        main_socket_url: MS,
        session_timeout: ST,
        gc_interval: GI,
    ) -> Result<MultiServer, ServerError>
    where
        MS: Into<MainSocketUrl<'s>>,
        ST: Into<SessionTimeout>,
        GI: Into<GcInterval>,
    {
        let main_url = main_socket_url.into();
        let session_timeout = session_timeout.into();
        let gc_interval = gc_interval.into();

        let handle = handle.clone();
        let mut nano_socket = NanoSocket::new(Protocol::Rep, &handle)?;

        // todo: implement lazy binding and main socket respawn
        nano_socket.bind(main_url.as_ref())?;

        let (writer, reader) = nano_socket.split();

        Ok(MultiServer {
            session_timeout: session_timeout.into(),
            gc_interval: gc_interval.into(),
            reader,
            writer,
            handle,
            connections: Rc::new(RefCell::new(HashMap::new())),
        })
    }

    fn next_connid(&self) -> ConnId {
        // naive, to be changed
        self.connections
            .borrow()
            .keys()
            .max()
            .map(|v| *v)
            .unwrap_or_default() + 1
    }
}

impl Stream for MultiServer {
    type Item = Message;
    type Error = ServerError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // poll main socket first

        //         match self.reader.poll() {
        //             Async::Ready => {
        //                 let message =
        //                 match self.send(&item) {
        //                     Ok(Some(_)) => Ok(AsyncSink::Ready),
        //                     Ok(None) => Ok(AsyncSink::NotReady(item)),
        //                     Err(err) => Err(err),
        //                 }
        //             }
        //             Async::NotReady => Ok(AsyncSink::NotReady(item)),
        //         }

        Ok(Async::NotReady)
    }
}

impl Sink for MultiServer {
    type SinkItem = Message;
    type SinkError = NanoError;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        Ok(AsyncSink::Ready)
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        trace!("MultiServer::Sink::poll_complete");

        Ok(Async::Ready(()))
    }
}
