#[macro_use]
extern crate log;

extern crate futures;
extern crate nanomsg;
extern crate nanomsg_tokio;
extern crate tokio_core;

extern crate bincode;
extern crate serde;
#[macro_use]
extern crate serde_derive;

mod error;
#[macro_use]
mod consts;
pub mod config;
pub mod proto;

use error::{PeerError, ServerError};
use consts::{INTERNAL_BUFFER_LENGTH, INTERNAL_PEER_BUFFER_LENGTH};
use config::{client_socket, GcInterval, MainSocketUrl, SessionTimeout};
use proto::{deserialize, serialize, ConnId, ControlReply, ControlRequest, PeerReply, PeerRequest};

use futures::{Async, AsyncSink, Future, Poll, Sink, StartSend, Stream};
use futures::unsync::oneshot::{channel as oneshot, Receiver as OneshotReceiver,
                               Sender as OneshotSender};
use futures::unsync::mpsc::{channel, Receiver, Sender};
use futures::sink::Buffer;

use nanomsg_tokio::Socket as NanoSocket;
use nanomsg::Protocol;

use tokio_core::reactor::{Handle, Interval};

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub type KillswitchSender = OneshotSender<()>;
pub type KillswitchReceiver = OneshotReceiver<()>;

#[derive(Debug)]
pub struct Session {
    pub connid: ConnId,
    pub socket_url: String,
    pub connection: PeerConnection,
}

impl Session {
    fn new(connid: ConnId, socket_url: String, connection: PeerConnection) -> Session {
        Session {
            connid,
            socket_url,
            connection,
        }
    }
}

#[derive(Debug)]
pub struct MultiServer {
    socket_url: String,
    session_timeout: SessionTimeout,
    gc_interval: GcInterval,
    sender: Sender<Session>,
    receiver: Option<Receiver<Session>>,
    handle: Handle,
    connections: HashMap<ConnId, OneshotSender<()>>,
}

pub struct MultiServerFutures {
    /// main server socket handler future
    /// this should be run on the io loop
    pub server: Box<Future<Item = (), Error = ServerError>>,
    /// a stream producing a `Session` for every incoming client session
    pub sessions: Receiver<Session>,
}

impl MultiServer {
    pub fn new<'s, MS, ST, GI>(
        main_socket_url: MS,
        session_timeout: ST,
        gc_interval: GI,
        handle: Handle,
    ) -> MultiServer
    where
        MS: Into<MainSocketUrl<'s>>,
        ST: Into<SessionTimeout>,
        GI: Into<GcInterval>,
    {
        let main_url = main_socket_url.into();
        let session_timeout = session_timeout.into();
        let gc_interval = gc_interval.into();

        let (sender, receiver) = channel(INTERNAL_BUFFER_LENGTH);

        MultiServer {
            socket_url: main_url.as_ref().to_owned(),
            session_timeout,
            gc_interval,
            sender,
            receiver: Some(receiver),
            handle,
            connections: HashMap::new(),
        }
    }

    pub fn into_futures(self) -> Result<MultiServerFutures, ServerError> {
        // create main REQ/REP socket
        let mut socket = NanoSocket::new(Protocol::Rep, &self.handle)?;

        // todo: implement lazy binding and main socket respawn
        socket.bind(self.socket_url.as_ref())?;

        let (writer, reader) = socket.split();

        let handle = self.handle.clone();
        let mut server = self;

        let receiver = server.receiver.take();

        Ok(MultiServerFutures {
            server: Box::new(
                reader
                    .map(move |message| {
                        let reply = match deserialize(&message) {
                            Ok(message) => {
                                let (reply, session) = server.process_message(message);

                                if let Some(session) = session {
                                    // session was properly established
                                    // send to session receiver stream

                                    handle.spawn(
                                        server.sender.clone().send(session).map(|_| ()).map_err(
                                            |err| {
                                                error!("Peer connection error: {:?}", err);
                                            },
                                        ),
                                    )
                                }

                                reply
                            }
                            Err(error) => {
                                let error = ServerError::from(error);

                                error!("Message error {:?}", error);

                                ControlReply::SocketCreated(Err(error.into()))
                            }
                        };

                        serialize(&reply).expect("Message serialization error")
                    })
                    .forward(writer)
                    .from_err()
                    .map(|_| ()),
            ),
            sessions: receiver.unwrap(),
        })
    }

    fn next_connid(&self) -> ConnId {
        // naive, to be changed
        self.connections.keys().max().cloned().unwrap_or_default() + 1
    }

    fn create_pair_socket(&mut self) -> Result<Session, ServerError> {
        let connid = self.next_connid();

        let client_url = client_socket(connid);

        info!("Creating Pair socket {}", &client_url);

        let (connection, killswitch) = PeerConnection::new(
            &client_url,
            self.session_timeout,
            self.gc_interval,
            &self.handle,
        )?;

        // register connection
        self.connections.insert(connid, killswitch);

        Ok(Session::new(connid, client_url, connection))
    }

    // Ok
    // - new socket -> session, control reply
    // - bad packet -> no session, control reply
    // Err

    #[inline]
    fn process_message(&mut self, message: ControlRequest) -> (ControlReply, Option<Session>) {
        match message {
            // not much choice here
            ControlRequest::CreateSocket => {
                let client_socket = self.create_pair_socket();

                let (reply, session) = match client_socket {
                    Ok(session) => (
                        ControlReply::SocketCreated(Ok((
                            session.connid,
                            session.socket_url.clone(),
                        ))),
                        Some(session),
                    ),
                    Err(e) => (ControlReply::SocketCreated(Err(e.into())), None),
                };

                (reply, session)
            }
        }
    }
}

pub struct PeerConnection {
    last_active: Instant,
    socket: Buffer<NanoSocket>,
    killswitch_receiver: Option<OneshotReceiver<()>>,
    gc: Interval,
    timeout: Duration,
}

impl PeerConnection {
    pub fn new<ST, GI>(
        url: &str,
        session_timeout: ST,
        gc_interval: GI,
        handle: &Handle,
    ) -> Result<(PeerConnection, OneshotSender<()>), PeerError>
    where
        ST: Into<SessionTimeout>,
        GI: Into<GcInterval>,
    {
        let session_timeout = session_timeout.into();
        let gc_interval = gc_interval.into();

        let mut socket = NanoSocket::new(Protocol::Pair, handle)?;

        socket.bind(url)?;

        let socket = socket.buffer(INTERNAL_PEER_BUFFER_LENGTH);

        debug!("Created Pair socket {:?}", url);

        // create killswitch
        let (sender, receiver) = oneshot();

        // spawn local GC interval
        let gc = Interval::new(gc_interval.into(), handle)?;

        Ok((
            PeerConnection {
                last_active: Instant::now(),
                socket,
                killswitch_receiver: Some(receiver),
                gc,
                timeout: session_timeout.into(),
            },
            sender,
        ))
    }

    #[inline]
    fn refresh(&mut self) {
        self.last_active = Instant::now();
    }

    #[inline]
    fn idle(&self) -> Duration {
        Instant::now() - self.last_active
    }
}

impl Stream for PeerConnection {
    type Item = PeerRequest;
    type Error = PeerError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // These two functions can be safely called only from within `poll()`
        // so it's best to declare them here

        #[inline]
        fn killswitch_activated(pc: &mut PeerConnection) -> bool {
            if let Some(killswitch) = pc.killswitch_receiver.as_mut() {
                match killswitch.poll() {
                    Ok(Async::Ready(..)) | Err(_) => true,
                    Ok(Async::NotReady) => false,
                }
            } else {
                // this shouldn't ever get called
                error!("PeerConnection Stream polled after the killswitch has been activated");
                unreachable!();
            }
        }

        #[inline]
        fn gc_timeout_expired(pc: &mut PeerConnection) -> Result<bool, ::std::io::Error> {
            if let Async::Ready(Some(_)) = pc.gc.poll()? {
                Ok(pc.idle() > pc.timeout)
            } else {
                Ok(false)
            }
        }

        if killswitch_activated(self) {
            // we should close the connection
            Ok(Async::Ready(None))
        }
        // next, poll gc interval
        else if gc_timeout_expired(self)? {
            // session timeout expired, close the connection
            debug!("Session timeout expired, closing {:?}", self);

            Ok(Async::Ready(None))
        } else {
            // poll main socket

            match self.socket.poll()? {
                Async::Ready(Some(data)) => {
                    let message = deserialize(&data)?;

                    // update keepalive information
                    self.refresh();

                    use PeerRequest::*;

                    match message {
                        Request(..) | Abort(_) | KeepAlive => Ok(Async::Ready(Some(message))),
                        CloseConnection => Ok(Async::Ready(None)),
                    }
                }
                Async::Ready(None) => Ok(Async::Ready(None)),
                Async::NotReady => Ok(Async::NotReady),
            }
        }
    }
}

impl Sink for PeerConnection {
    type SinkItem = PeerReply;
    type SinkError = PeerError;

    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        let message = serialize(&item)?;

        match self.socket.start_send(message) {
            Ok(AsyncSink::NotReady(_message)) => {
                // this indicates that we need buffering
                // for now return error and simply drop the message
                Err(PeerError::BufferOverflow)
            }
            Ok(AsyncSink::Ready) => Ok(AsyncSink::Ready),
            Err(err) => Err(err)?,
        }
    }

    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        trace!("PeerConnection::Sink::poll_complete");

        self.socket.poll_complete().map_err(|e| e.into())
    }
}

// we have to do it manually, because Interval doesn't #[derive(Debug)]

use std::fmt::{Debug, Error as FmtError, Formatter};

impl Debug for PeerConnection {
    fn fmt(&self, f: &mut Formatter) -> Result<(), FmtError> {
        f.debug_struct("PeerConnection")
            .field("last_active", &self.last_active)
            .field("socket", &self.socket)
            .field("killswitch_receiver", &self.killswitch_receiver)
            .field("gc", &"Interval")
            .field("timeout", &self.timeout)
            .finish()
    }
}

impl Drop for PeerConnection {
    fn drop(&mut self) {
        debug!("Dropping PeerConnection {:?}", self);
    }
}
