extern crate colored_logger;
extern crate flexi_logger;
extern crate futures;
#[macro_use]
extern crate log;
extern crate nanomsg;
extern crate nanomsg_multi_server;
extern crate nanomsg_tokio;
extern crate rand;
extern crate tokio_core;

use tokio_core::reactor::{Core, Interval};
use nanomsg_multi_server::proto::{deserialize, serialize, ControlReply, ControlRequest};
use nanomsg_tokio::Socket as NanoSocket;
use nanomsg::Protocol;

use std::time::Duration;

use futures::{Future, Sink, Stream};

const MAIN_SOCKET_URL: &str = "ipc:///tmp/nanoserver-main.ipc";

fn main() {
    flexi_logger::Logger::with_env()
        .format(colored_logger::formatter)
        .start()
        .expect("Logger initialization failed");

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    info!("Starting socket server");

    let mut nano_socket = NanoSocket::new(Protocol::Req, &handle).expect("Unable to create socket");

    nano_socket
        .connect(MAIN_SOCKET_URL)
        .expect("Unable to connect to main socket");

    let (writer, reader) = nano_socket.split();

    let connect_request = ControlRequest::CreateSocket;
    let connect_request = serialize(&connect_request).unwrap();

    let client = {
        let handle = handle.clone();

        writer
            .send(connect_request)
            .map_err(|err| error!("Error {:?}", err))
            .and_then(move |_| {
                reader
                    .into_future()
                    .map(move |reply| {
                        let reply = deserialize(reply.0.as_ref().unwrap()).unwrap();

                        let ControlReply::SocketCreated(result) = reply;
                        let (_connid, sockurl) = result.unwrap();

                        info!("Attempting to connect to: {}", sockurl);

                        let mut socket = NanoSocket::new(Protocol::Pair, &handle)
                            .expect("Unable to create client socket");

                        socket
                            .connect(&sockurl)
                            .expect("Unable to connect to client socket");

                        let (writer, reader) = socket.split();

                        handle.spawn(
                            reader
                                .for_each(|msg| {
                                    info!("Received reply: {:?}", msg);

                                    Ok(())
                                })
                                .map_err(|_| ()),
                        );

                        handle.spawn(
                            Interval::new(Duration::from_secs(3), &handle)
                                .unwrap()
                                .map(|_| rand::random::<u64>().to_string().as_bytes().to_vec())
                                .map_err(|_| ())
                                .forward(writer.sink_map_err(|_| ()))
                                .map(|_| ())
                                .map_err(|_| ()),
                        );

                        Ok::<(), ()>(())
                    })
                    .map(|_| ())
                    .map_err(|_| ())
            })
    };

    core.run(client).unwrap();
}
